use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
crate struct CreateInstanceRequest {
    crate name: String,
    crate cpu: usize,
    crate memory: usize,
    crate disk_size: usize,
    crate image: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
crate struct UpdateInstanceRequest {
    crate cpu: usize,
    crate memory: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
crate struct Instance {
    crate name: String,
    crate cpu: usize,
    crate memory: usize,
    crate disk_size: usize,
    crate hostname: String,
    crate ssh_host: Option<String>,
    crate ssh_port: Option<i32>,
    crate password: String,
    crate status: String,
    crate image: String,
}

fn strip_image_tag(image: String) -> String {
    if let Some(i) = image.rfind(':') {
        image[..i].to_string()
    } else {
        image
    }
}

impl From<&crate::model::Instance> for Instance {
    fn from(m: &crate::model::Instance) -> Self {
        Instance {
            name: m.name.clone(),
            cpu: m.cpu,
            memory: m.memory,
            disk_size: m.disk_size,
            hostname: m.hostname.clone(),
            ssh_host: m.ssh_host.clone(),
            ssh_port: m.ssh_port,
            password: m.password.clone(),
            status: m.status.to_string(),
            image: strip_image_tag(m.image.clone()),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
crate struct ListInstancesResponse {
    crate instances: Vec<Instance>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_image_tag() {
        assert_eq!(
            strip_image_tag("tispace/ubuntu2004".to_owned()),
            "tispace/ubuntu2004".to_owned()
        );
        assert_eq!(
            strip_image_tag("tispace/ubuntu2004:latest".to_owned()),
            "tispace/ubuntu2004".to_owned()
        );
        assert_eq!(
            strip_image_tag("tispace/ubuntu2004:1.2.0".to_owned()),
            "tispace/ubuntu2004".to_owned()
        );
    }
}
