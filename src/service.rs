use crypto::pbkdf2::{pbkdf2_check, pbkdf2_simple};
use hyper::body::Buf;
use hyper::{Body, Method, Request, Response, StatusCode};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use percent_encoding::percent_decode;

use crate::error::*;
use crate::model::{
    ChangePasswordRequest, CreateInstanceRequest, CreateInstanceResponse, ErrorResponse,
    Instance as HttpInstance, ListInstancesResponse, UserClaims, UserLoginRequest,
    UserLoginResponse,
};
use crate::storage::{Instance, InstanceStage, InstanceStatus, Storage};

fn path_escape(path: &str) -> Result<String> {
    Ok(percent_decode(path.as_bytes()).decode_utf8()?.into())
}

fn write_code(code: StatusCode) -> Result<Response<Body>> {
    Ok(Response::builder().status(code).body("".into()).unwrap())
}

fn write_error(code: StatusCode, message: &str) -> Result<Response<Body>> {
    let resp = ErrorResponse {
        error: message.to_string(),
    };
    let body = serde_json::to_string(&resp).unwrap();
    Ok(Response::builder().status(code).body(body.into()).unwrap())
}

pub struct HttpService {
    storage: Storage,
    secret: String,
}

impl HttpService {
    pub fn new(storage: Storage, secret: String) -> Self {
        HttpService { storage, secret }
    }

    pub async fn home(&self, _req: Request<Body>) -> Result<Response<Body>> {
        Ok(Response::new("Hello, World!!!".into()))
    }

    pub async fn user_login(&self, req: Request<Body>) -> Result<Response<Body>> {
        let body = hyper::body::aggregate(req).await?;
        let params: UserLoginRequest = serde_json::from_reader(body.reader())?;
        if params.username.is_empty() {
            return write_error(StatusCode::BAD_REQUEST, "username is empty");
        }
        if params.password.is_empty() {
            return write_error(StatusCode::BAD_REQUEST, "password is empty");
        }
        let mut verified = false;
        self.storage
            .read_only(|state| {
                verified = state.users.iter().any(|u| {
                    u.username == params.username
                        && pbkdf2_check(&params.password, &u.password_hash)
                            .ok()
                            .unwrap()
                });
            })
            .await;
        if verified {
            let claims = UserClaims {
                username: params.username,
            };
            let token = encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(self.secret.as_bytes()),
            )?;
            let resp = UserLoginResponse { token };
            let body = serde_json::to_string(&resp).unwrap();
            Ok(Response::new(body.into()))
        } else {
            write_code(StatusCode::UNAUTHORIZED)
        }
    }

    fn verify_auth(&self, req: &Request<Body>) -> Option<String> {
        let auth = req.headers().get("Authorization")?.to_str().ok()?;
        let token = auth.split_whitespace().last()?;
        // FIXME: It seems the usage is not correct yet.
        let claims = decode::<UserClaims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &Validation::default(),
        )
        .ok()?;
        Some(claims.claims.username)
    }

    pub async fn change_password(&self, req: Request<Body>) -> Result<Response<Body>> {
        let username = self.verify_auth(&req);
        if username.is_none() {
            return write_code(StatusCode::UNAUTHORIZED);
        }
        let username = username.unwrap();
        let body = hyper::body::aggregate(req).await?;
        let params: ChangePasswordRequest = serde_json::from_reader(body.reader())?;
        if params.new_password.is_empty() {
            return write_error(StatusCode::BAD_REQUEST, "new_password is empty");
        }
        self.storage
            .read_write(
                |state| match state.users.iter_mut().find(|u| u.username == username) {
                    Some(u) => {
                        u.password_hash = pbkdf2_simple(&params.new_password, 1024).unwrap();
                        true
                    }
                    None => false,
                },
            )
            .await?;
        write_code(StatusCode::NO_CONTENT)
    }

    pub async fn create_instance(&self, req: Request<Body>) -> Result<Response<Body>> {
        let username = self.verify_auth(&req);
        if username.is_none() {
            return write_code(StatusCode::UNAUTHORIZED);
        }
        let username = username.unwrap();
        let body = hyper::body::aggregate(req).await?;
        let params: CreateInstanceRequest = serde_json::from_reader(body.reader())?;
        if params.name.is_empty() {
            return write_error(StatusCode::BAD_REQUEST, "name is empty");
        }
        if params.cpu == 0 {
            return write_error(StatusCode::BAD_REQUEST, "cpu must be greater than 0");
        }
        if params.memory == 0 {
            return write_error(StatusCode::BAD_REQUEST, "memory must be greater than 0");
        }
        if params.disk_size == 0 {
            return write_error(StatusCode::BAD_REQUEST, "disk_size must be greater than 0");
        }
        let domain_name = format!("{}.tispace.{}.svc.cluster.local", params.name, username);

        let mut already_exists = false;
        let mut quota_exceeded = false;
        let mut created = false;
        self.storage
            .read_write(
                |state| match state.users.iter_mut().find(|u| u.username == username) {
                    Some(u) => {
                        if u.instances.len() + 1 > u.instance_quota {
                            quota_exceeded = true;
                            return false;
                        }
                        let mut total_cpu = 0;
                        let mut total_memory = 0;
                        let mut total_disk_size = 0;
                        for instance in &mut u.instances {
                            if instance.name == params.name {
                                already_exists = true;
                                return false;
                            }
                            total_cpu += instance.cpu;
                            total_memory += instance.memory;
                            total_disk_size += instance.disk_size;
                        }
                        quota_exceeded = total_cpu + params.cpu > u.cpu_quota
                            || total_memory + params.memory > u.memory_quota
                            || total_disk_size + params.disk_size > u.disk_quota;
                        if quota_exceeded {
                            return false;
                        }

                        u.instances.push(Instance {
                            name: params.name.clone(),
                            cpu: params.cpu,
                            memory: params.memory,
                            disk_size: params.disk_size,
                            stage: InstanceStage::Pending,
                            domain_name: domain_name.clone(),
                            status: InstanceStatus::Pending,
                        });
                        created = true;
                        created
                    }
                    None => false,
                },
            )
            .await?;

        if already_exists {
            write_code(StatusCode::CONFLICT)
        } else if quota_exceeded {
            write_code(StatusCode::UNPROCESSABLE_ENTITY)
        } else if created {
            let resp = CreateInstanceResponse { domain_name };
            let body = serde_json::to_string(&resp).unwrap();
            Ok(Response::builder()
                .status(StatusCode::CREATED)
                .body(body.into())
                .unwrap())
        } else {
            write_code(StatusCode::UNAUTHORIZED)
        }
    }

    pub async fn delete_instance(&self, req: Request<Body>) -> Result<Response<Body>> {
        let username = self.verify_auth(&req);
        if username.is_none() {
            return write_code(StatusCode::UNAUTHORIZED);
        }
        let username = username.unwrap();
        let instance_name = path_escape(req.uri().path())?
            .strip_prefix("/user/instances/")
            .unwrap()
            .to_string();

        self.storage
            .read_write(
                |state| match state.users.iter_mut().find(|u| u.username == username) {
                    Some(u) => {
                        match u.instances.iter_mut().find(|instance| {
                            instance.name == instance_name
                                && instance.stage != InstanceStage::Deleting
                        }) {
                            Some(instance) => {
                                instance.stage = InstanceStage::Deleting;
                                true
                            }
                            None => false,
                        }
                    }
                    None => false,
                },
            )
            .await?;
        write_code(StatusCode::NO_CONTENT)
    }

    pub async fn list_instances(&self, req: Request<Body>) -> Result<Response<Body>> {
        let username = self.verify_auth(&req);
        if username.is_none() {
            return write_code(StatusCode::UNAUTHORIZED);
        }
        let username = username.unwrap();
        let mut http_instances = Vec::new();
        self.storage
            .read_only(
                |state| match state.users.iter().find(|&u| u.username == username) {
                    Some(u) => {
                        http_instances = u
                            .instances
                            .iter()
                            .map(|instance| HttpInstance {
                                name: instance.name.clone(),
                                cpu: instance.cpu,
                                memory: instance.memory,
                                disk_size: instance.disk_size,
                                domain_name: instance.domain_name.clone(),
                                status: instance.status.to_string(),
                            })
                            .collect();
                    }
                    None => (),
                },
            )
            .await;
        let resp = ListInstancesResponse {
            instances: http_instances,
        };
        let body = serde_json::to_string(&resp).unwrap();
        Ok(Response::new(body.into()))
    }

    pub async fn serve_http(&self, req: Request<Body>) -> Result<Response<Body>> {
        let result = match (req.method(), req.uri().path()) {
            (&Method::GET, "/") | (&Method::GET, "/index.html") => self.home(req).await,
            (&Method::POST, "/user/login") => self.user_login(req).await,
            (&Method::PUT, "/user/password") => self.change_password(req).await,
            (&Method::POST, "/user/instances") => self.create_instance(req).await,
            (&Method::DELETE, path)
                if path.starts_with("/user/instances/")
                    && !path.strip_prefix("/user/instances/").unwrap().is_empty() =>
            {
                self.delete_instance(req).await
            }
            (&Method::GET, "/user/instances") => self.list_instances(req).await,
            _ => write_code(StatusCode::NOT_FOUND),
        };
        match result {
            Ok(resp) => Ok(resp),
            Err(e) => write_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string().as_str()),
        }
    }
}
