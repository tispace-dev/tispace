use hyper::body::Buf;
use hyper::{Body, Method, Request, Response, StatusCode};

use crypto::pbkdf2::{pbkdf2_check, pbkdf2_simple};
use percent_encoding::percent_decode;

use crate::error::*;
use crate::model::{
    ChangePasswordRequest, CreateInstanceRequest, CreateInstanceResponse, ErrorResponse,
    Instance as HttpInstance, ListInstancesResponse, UserLoginRequest, UserLoginResponse,
};
use crate::storage::{Instance, InstanceStage, InstanceStatus, Storage};

fn path_escape(path: &str) -> Result<String> {
    Ok(percent_decode(path.as_bytes()).decode_utf8()?.into())
}

fn write_code(code: StatusCode) -> Result<Response<Body>> {
    Ok(Response::builder().status(code).body("".into()).unwrap())
}

fn write_error(code: StatusCode, message: &str) -> Result<Response<Body>> {
    let resp_json = ErrorResponse {
        error: message.to_string(),
    };
    let body = serde_json::to_string(&resp_json).unwrap();
    Ok(Response::builder().status(code).body(body.into()).unwrap())
}

pub struct HttpService {
    storage: Storage,
}

impl HttpService {
    pub fn new(storage: Storage) -> Self {
        HttpService { storage }
    }

    pub async fn home(&self, _req: Request<Body>) -> Result<Response<Body>> {
        Ok(Response::new("Hello, World!!!".into()))
    }

    pub async fn user_login(&self, req: Request<Body>) -> Result<Response<Body>> {
        let body = hyper::body::aggregate(req).await?;
        let req_json: UserLoginRequest = serde_json::from_reader(body.reader())?;
        if req_json.username.is_empty() {
            return write_error(StatusCode::BAD_REQUEST, "username is empty");
        }
        if req_json.password.is_empty() {
            return write_error(StatusCode::BAD_REQUEST, "password is empty");
        }
        let mut verified = false;
        self.storage.read_only(|state| {
            verified = state.users.iter().any(|u| {
                u.username == req_json.username
                    && pbkdf2_check(&req_json.password, &u.password_hash)
                        .ok()
                        .unwrap()
            });
        });
        if verified {
            // FIXME: token should encrypted with a secret key.
            let token = base64::encode(serde_json::to_string(&req_json.username).unwrap());
            let resp_json = UserLoginResponse { token };
            let body = serde_json::to_string(&resp_json).unwrap();
            Ok(Response::new(body.into()))
        } else {
            write_code(StatusCode::UNAUTHORIZED)
        }
    }

    fn verify_credentials(&self, credentials: &[u8]) -> Option<String> {
        let mut colon = credentials.len();
        for i in 0..credentials.len() {
            if credentials[i] == b':' {
                colon = i;
                break;
            }
        }
        if colon == credentials.len() {
            return None;
        }
        let username = String::from_utf8_lossy(&credentials[..colon]).to_string();
        // FIXME: verify token.
        let mut found = false;
        self.storage
            .read_only(|state| found = state.users.iter().any(|u| u.username == username));
        Some(username)
    }

    fn verify_basic_auth(&self, req: &Request<Body>) -> Option<String> {
        let auth = req.headers().get("Authorization")?.to_str().ok()?;
        let credentials = base64::decode(auth.split_whitespace().last()?).ok()?;
        self.verify_credentials(&credentials)
    }

    pub async fn change_password(&self, req: Request<Body>) -> Result<Response<Body>> {
        let username = self.verify_basic_auth(&req);
        if username.is_none() {
            return write_code(StatusCode::UNAUTHORIZED);
        }
        let username = username.unwrap();
        let body = hyper::body::aggregate(req).await?;
        let req_json: ChangePasswordRequest = serde_json::from_reader(body.reader())?;
        if req_json.new_password.is_empty() {
            return write_error(StatusCode::BAD_REQUEST, "new_password is empty");
        }
        self.storage
            .read_write(|state| {
                for u in &mut state.users {
                    if u.username == username {
                        u.password_hash = pbkdf2_simple(&req_json.new_password, 1024).unwrap();
                        return true;
                    }
                }
                false
            })
            .await?;
        write_code(StatusCode::NO_CONTENT)
    }

    pub async fn create_instance(&self, req: Request<Body>) -> Result<Response<Body>> {
        let username = self.verify_basic_auth(&req);
        if username.is_none() {
            return write_code(StatusCode::UNAUTHORIZED);
        }
        let username = username.unwrap();
        let body = hyper::body::aggregate(req).await?;
        let req_json: CreateInstanceRequest = serde_json::from_reader(body.reader())?;
        if req_json.name.is_empty() {
            return write_error(StatusCode::BAD_REQUEST, "name is empty");
        }
        if req_json.cpu == 0 {
            return write_error(StatusCode::BAD_REQUEST, "cpu must be greater than 0");
        }
        if req_json.memory == 0 {
            return write_error(StatusCode::BAD_REQUEST, "memory must be greater than 0");
        }
        if req_json.disk_size == 0 {
            return write_error(StatusCode::BAD_REQUEST, "disk_size must be greater than 0");
        }
        let domain_name = format!("{}.tispace.{}.svc.cluster.local", req_json.name, username);

        let mut already_exists = false;
        let mut quota_exceeded = false;
        let mut created = false;
        self.storage
            .read_write(|state| {
                for u in &mut state.users {
                    if u.username == username {
                        if u.instances.len() + 1 > u.instance_quota {
                            quota_exceeded = true;
                            return false;
                        }
                        let mut total_cpu = 0;
                        let mut total_memory = 0;
                        let mut total_disk_size = 0;
                        for instance in &mut u.instances {
                            if instance.name == req_json.name {
                                already_exists = true;
                                return false;
                            }
                            total_cpu += instance.cpu;
                            total_memory += instance.memory;
                            total_disk_size += instance.disk_size;
                        }
                        quota_exceeded = total_cpu + req_json.cpu > u.cpu_quota
                            || total_memory + req_json.memory > u.memory_quota
                            || total_disk_size + req_json.disk_size > u.disk_quota;
                        if quota_exceeded {
                            return false;
                        }

                        u.instances.push(Instance {
                            name: req_json.name.clone(),
                            cpu: req_json.cpu,
                            memory: req_json.memory,
                            disk_size: req_json.disk_size,
                            stage: InstanceStage::Pending,
                            domain_name: domain_name.clone(),
                            status: InstanceStatus::Pending,
                        });
                        created = true;
                        return true;
                    }
                }
                false
            })
            .await?;

        if already_exists {
            write_code(StatusCode::CONFLICT)
        } else if quota_exceeded {
            write_code(StatusCode::UNPROCESSABLE_ENTITY)
        } else if created {
            let resp_json = CreateInstanceResponse { domain_name };
            let body = serde_json::to_string(&resp_json).unwrap();
            Ok(Response::builder()
                .status(StatusCode::CREATED)
                .body(body.into())
                .unwrap())
        } else {
            write_code(StatusCode::UNAUTHORIZED)
        }
    }

    pub async fn delete_instance(&self, req: Request<Body>) -> Result<Response<Body>> {
        let username = self.verify_basic_auth(&req);
        if username.is_none() {
            return write_code(StatusCode::UNAUTHORIZED);
        }
        let username = username.unwrap();
        let instance_name = path_escape(req.uri().path())?
            .strip_prefix("/user/instances/")
            .unwrap()
            .to_string();

        self.storage
            .read_write(|state| {
                for u in &mut state.users {
                    if u.username == username {
                        for instance in &mut u.instances {
                            if instance.name == instance_name {
                                return if instance.stage != InstanceStage::Deleting {
                                    instance.stage = InstanceStage::Deleting;
                                    true
                                } else {
                                    false
                                };
                            }
                        }
                        return false;
                    }
                }
                false
            })
            .await?;
        write_code(StatusCode::NO_CONTENT)
    }

    pub async fn list_instances(&self, req: Request<Body>) -> Result<Response<Body>> {
        let username = self.verify_basic_auth(&req);
        if username.is_none() {
            return write_code(StatusCode::UNAUTHORIZED);
        }
        let username = username.unwrap();
        let mut http_instances = Vec::new();
        self.storage.read_only(|state| {
            for u in &state.users {
                if u.username == username {
                    for instance in &u.instances {
                        http_instances.push(HttpInstance {
                            name: instance.name.clone(),
                            cpu: instance.cpu,
                            memory: instance.memory,
                            disk_size: instance.disk_size,
                            domain_name: instance.domain_name.clone(),
                            status: instance.status.to_string(),
                        })
                    }
                    return;
                }
            }
        });
        let resp_json = ListInstancesResponse {
            instances: http_instances,
        };
        let body = serde_json::to_string(&resp_json).unwrap();
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
