use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::Path;

use derive_getters::Getters;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Getters, Debug)]
pub struct Squishfile {
    run: Run,
    layers: HashMap<String, String>, // TODO: <String, Version>
    env: HashMap<String, String>,
    ports: Vec<Port>,
}

impl Squishfile {
    fn update_layer(&mut self, layer: &String, new_data: &String) {
        self.layers.insert(layer.to_string(), new_data.to_string());
    }

    pub fn resolve_paths(&mut self) {
        let resolved: Vec<(String, String)> = self
            .layers
            .iter()
            .filter(|(_k, v)| v.starts_with("./")) // TODO: Handle other relative paths
            .map(|(k, v)| {
                let path = fs::canonicalize(v).unwrap(); // TODO: Handle this panic
                (
                    k.clone(),
                    path.to_str().expect("no string from path!?").to_string(),
                )
            })
            .collect();

        resolved.iter().for_each(|(k, v)| self.update_layer(k, v));
    }

    pub fn to_json(&self) -> Result<String, Box<dyn Error>> {
        serde_json::to_string(&self).map_err(|e| e.into())
    }

    pub fn from_json<'a, S: Into<&'a str>>(json: S) -> Result<Self, Box<dyn Error>> {
        serde_json::from_str(json.into()).map_err(|e| e.into())
    }
}

impl Into<String> for Squishfile {
    fn into(self) -> String {
        toml::to_string(&self).expect(format!("unable to serialise config: {:?}", self).as_str())
    }
}

#[derive(Deserialize, Serialize, Getters, Debug)]
pub struct Run {
    command: String,
    args: Vec<String>,
}

#[derive(Deserialize, Serialize, Getters, Debug)]
pub struct Port {
    container: u16,
    host: u16,
    protocol: PortProtocol,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum PortProtocol {
    Tcp,
    Udp,
}

pub fn parse_str<'a, T: Into<&'a str>>(squishfile: T) -> Result<Squishfile, Box<dyn Error>> {
    toml::from_str::<Squishfile>(squishfile.into()).map_err(|e| e.into())
}

pub fn parse<T: AsRef<Path>>(squishfile: T) -> Result<Squishfile, Box<dyn Error>> {
    let content = fs::read_to_string(squishfile)?;
    parse_str(&*content)
}
