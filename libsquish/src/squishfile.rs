use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fs;
use std::path::Path;

use derive_getters::Getters;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Getters, Debug)]
pub struct Squishfile {
    run: Run,
    layers: BTreeMap<String, LayerSpec>,
    env: HashMap<String, String>,
    ports: Vec<Port>,
}

impl Squishfile {
    fn update_layer(&mut self, layer: &String, new_data: &LayerSpec) {
        self.layers.insert(layer.to_string(), new_data.clone());
    }

    pub fn resolve_paths(&mut self) {
        let resolved: Vec<(String, LayerSpec)> = self
            .layers
            .iter()
            // Easiest way to detect local paths -- generic labels means we
            // can't resolve every possible path
            .filter(|(_k, v)| match v {
                &&LayerSpec {
                    version: None,
                    path: Some(_),
                    target: _,
                } => true,
                _ => false,
            })
            // This is safe because we just checked it
            .map(|(k, v)| match fs::canonicalize(v.path.as_ref().unwrap()) {
                Ok(path) => {
                    let path = path.as_path().display().to_string();
                    let new_target = match &v.target {
                        Some(target) => Some(target.clone()),
                        None => None,
                    };
                    (
                        k.clone(),
                        LayerSpec {
                            version: None,
                            path: Some(path),
                            target: new_target,
                        },
                    )
                }
                Err(e) => panic!("squishfile: error resolving relative path: {}", e),
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

#[derive(Deserialize, Serialize, Getters, Debug, Clone)]
pub struct LayerSpec {
    version: Option<String>,
    path: Option<String>,
    target: Option<String>,
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
    let mut parsed: toml::Value = toml::from_str(squishfile.into())?;
    let table = match parsed.as_table_mut() {
        Some(table) => table,
        None => panic!("squishfile: root not table"),
    };
    let run: Run = match table.get("run") {
        Some(run) => run.clone().try_into()?,
        None => panic!("squishfile: run not found"),
    };
    let env: HashMap<String, String> = match table.get("env") {
        Some(env) => env.clone().try_into()?,
        None => HashMap::new(),
    };
    let ports: Vec<Port> = match table.get("ports") {
        // TODO: Handle error better
        Some(ports) => ports
            .as_array()
            .expect("ports not array")
            .iter()
            .map(|p| p.clone().try_into::<Port>().unwrap())
            .collect(),
        None => vec![],
    };
    let layers: BTreeMap<String, LayerSpec> = match table.get("layers") {
        Some(layers) => {
            match layers.as_table() {
                Some(layers) => {
                    let mut map = BTreeMap::new();
                    layers
                        .iter()
                        .map(|(k, v)| {
                            let out = if let Some(maybe_path) = v.as_str() {
                                if maybe_path.starts_with("./") || maybe_path.starts_with("../") {
                                    LayerSpec {
                                        version: None,
                                        path: Some(maybe_path.to_string()),
                                        target: None,
                                    }
                                } else {
                                    LayerSpec {
                                        version: Some(maybe_path.to_string()),
                                        path: None,
                                        target: None,
                                    }
                                }
                            } else {
                                // TODO: Better error handling
                                v.clone().try_into::<LayerSpec>().unwrap()
                            };
                            (k, out)
                        })
                        .for_each(|(k, v)| {
                            map.insert(k.clone(), v);
                        });
                    map
                }
                None => panic!("squishfile: layers: not table"),
            }
        }
        None => panic!("squishfile: layers not found"),
    };

    Ok(Squishfile {
        layers,
        run,
        env,
        ports,
    })
}

pub fn parse<T: AsRef<Path>>(squishfile: T) -> Result<Squishfile, Box<dyn Error>> {
    let content = fs::read_to_string(squishfile)?;
    parse_str(&*content)
}
