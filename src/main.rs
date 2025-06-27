use anyhow::{anyhow, Result};
use clap::{Parser, ValueEnum};
use erased_serde;
use serde::de::DeserializeOwned;
use std::{
    io::{stdin, stdout, Read, Write},
    path::PathBuf,
    str,
};
use tera::Tera;

pub mod systemd;
use systemd::{process_systemd_configs, SystemdUnits};

pub mod utils;
use utils::{print_files, write_files};

#[derive(Parser, Debug)]
#[clap(name = "yamlaters", version = "0.1.0", author = "squirreljetpack")]
pub struct Opts {
    #[clap(flatten)]
    pub file_cmd: FileCmd,
}

#[derive(Parser, Debug)]
#[clap(name = "yamlaters")]
pub struct FileCmd {
    // if no input is given, then switch to console mode
    pub input: Option<PathBuf>,
    #[clap(short, long)]
    pub output: Option<PathBuf>,
    #[clap(short, long, value_enum)]
    pub from: Option<FromVariant>,
    #[clap(short, long, value_enum)]
    pub to: Option<ToVariant>,
    #[clap(long)]
    pub tera: bool,
    #[clap(short, long)]
    pub verbose: bool,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum FromVariant {
    Json,
    Yaml,
    Cbor,
    Ron,
    Toml,
    Bson,
}

impl FromVariant {
    // Deserialize into a struct
    pub fn deserialize_into<T>(&self, s: &[u8]) -> Result<T>
    where
        T: DeserializeOwned,
    {
        match self {
            FromVariant::Json => serde_json::from_slice(s).map_err(anyhow::Error::new),
            FromVariant::Yaml => serde_yaml::from_slice(s).map_err(anyhow::Error::new),
            FromVariant::Cbor => serde_cbor::from_slice(s).map_err(anyhow::Error::new),
            FromVariant::Ron => ron::de::from_bytes(s).map_err(anyhow::Error::new),
            FromVariant::Toml => {
                let s = str::from_utf8(s)?;
                toml::from_str(s).map_err(anyhow::Error::new)
            }
            FromVariant::Bson => bson::from_slice(s).map_err(anyhow::Error::new),
        }
    }

    // Run a callback on deserialized object without intermediate Box
    fn serialize<T>(&self, input: Vec<u8>, s: T)
    where
        T: Fn(&dyn erased_serde::Serialize),
    {
        match self {
            FromVariant::Json => {
                let v = serde_json::from_slice::<serde_json::Value>(&input).unwrap();
                s(&v);
            }
            FromVariant::Yaml => {
                let v = serde_yaml::from_slice::<serde_yaml::Value>(&input).unwrap();
                s(&v);
            }
            FromVariant::Cbor => {
                let v = serde_cbor::from_slice::<serde_cbor::Value>(&input).unwrap();
                s(&v);
            }
            FromVariant::Ron => {
                let v = ron::de::from_bytes::<ron::Value>(&input).unwrap();
                s(&v);
            }
            FromVariant::Toml => {
                let st = str::from_utf8(&input).unwrap();
                let v = toml::from_str::<toml::Value>(st).unwrap();
                s(&v);
            }
            FromVariant::Bson => {
                let v = bson::from_slice::<bson::Bson>(&input).unwrap();
                s(&v);
            }
        }
    }
}

impl From<FromVariant> for ToVariant {
    fn from(variant: FromVariant) -> Self {
        match variant {
            FromVariant::Json => ToVariant::Json,
            FromVariant::Yaml => ToVariant::Yaml,
            FromVariant::Cbor => ToVariant::Cbor,
            FromVariant::Ron => ToVariant::Ron,
            FromVariant::Toml => ToVariant::Toml,
            FromVariant::Bson => ToVariant::Bson,
        }
    }
}

// Get Variant from filepath
impl From<&PathBuf> for FromVariant {
    fn from(path: &PathBuf) -> Self {
        let p = path
            .extension()
            .expect("Extension not found, the type of the file could not be inferred.");
        match p.to_str().unwrap() {
            "bson" | "bs" => FromVariant::Bson,
            "cbor" | "cb" => FromVariant::Cbor,
            "json" => FromVariant::Json,
            "ron" => FromVariant::Ron,
            "toml" | "service" => FromVariant::Toml,
            "yaml" | "yml" => FromVariant::Yaml,
            _ => panic!("Type of the file could not be inferred"),
        }
    }
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq)]
pub enum ToVariant {
    Pickle,
    Bincode,
    Postcard,
    Flexbuffers,
    Json,
    PrettyJson,
    Yaml,
    Cbor,
    Ron,
    PrettyRon,
    Toml,
    Bson,
    Systemd,
    Quadlet,
}


impl ToVariant {
    fn from_path(path: &PathBuf) -> Option<Self> {
        let p = path.extension()?.to_str()?;
        match p {
            "bincode" | "bc" => Some(Self::Bincode),
            "bson" | "bs" => Some(Self::Bson),
            "cbor" | "cb" => Some(Self::Cbor),
            "yaml" | "yml" => Some(Self::Yaml),
            "flexbuffers" | "fb" => Some(Self::Flexbuffers),
            "postcard" | "pc" => Some(Self::Postcard),
            "pickle" | "pkl" => Some(Self::Pickle),
            "json" => Some(Self::Json),
            "hjson" => Some(Self::PrettyJson),
            "ron" => Some(Self::Ron),
            "hron" => Some(Self::PrettyRon),
            "toml" => Some(Self::Toml),
            "service" | "unit" => Some(Self::Systemd),
            "pod" => Some(Self::Quadlet),
            _ => None,
        }
    }

    fn to_buf(self, obj: &dyn erased_serde::Serialize) -> Vec<u8> {
        match self {
            ToVariant::Pickle => {
                serde_pickle::to_vec(&obj, serde_pickle::SerOptions::new()).unwrap()
            }
            ToVariant::Bincode => bincode::serialize(&obj).unwrap(),
            ToVariant::Postcard => postcard::to_allocvec(&obj).unwrap(),
            ToVariant::Flexbuffers => flexbuffers::to_vec(&obj).unwrap(),
            ToVariant::Json => serde_json::to_vec(&obj).unwrap(),
            ToVariant::PrettyJson => serde_json::to_vec_pretty(&obj).unwrap(),
            ToVariant::Yaml => serde_yaml::to_string(&obj).unwrap().into_bytes(),
            ToVariant::Cbor => serde_cbor::to_vec(&obj).unwrap(),
            ToVariant::Ron => ron::to_string(&obj).unwrap().into_bytes(),
            ToVariant::PrettyRon => {
                let s = ron::ser::PrettyConfig::new();
                let s = ron::ser::to_string_pretty(&obj, s).unwrap();
                s.into_bytes()
            }
            ToVariant::Toml => toml::to_string(&obj).unwrap().into_bytes(),
            ToVariant::Bson => bson::to_vec(&obj).unwrap(),

            _ => {
                panic!("Special variants have custom handling.")
            }
        }
    }
}

pub fn run(opts: Opts) -> Result<()> {
    let file_cmd = opts.file_cmd;
    let input = file_cmd.input;
    let from = file_cmd.from;
    let to = file_cmd.to;
    let output = file_cmd.output;
    let tera_enabled = file_cmd.tera;
    let verbose_enabled = file_cmd.verbose;

    let from_variant: FromVariant;
    let mut input_bytes = Vec::new();

    match input {
        Some(inp_path) => {
            input_bytes = std::fs::read(&inp_path)?;
            from_variant = from.unwrap_or_else(|| FromVariant::from(&inp_path));
        }
        None => {
            stdin().lock().read_to_end(&mut input_bytes)?;
            from_variant = from.ok_or_else(|| {
                anyhow!("Input format must be specified with --from when reading from stdin")
            })?;
        }
    }

    let to_variant = to.unwrap_or_else(|| {
        output
            .as_ref()
            .and_then(ToVariant::from_path)
            .unwrap_or_else(|| from_variant.into())
    });

    if tera_enabled {
        let input_str = str::from_utf8(&input_bytes)?;
        let context= tera::Context::new();
        let rendered = Tera::one_off(input_str, &context, true)?;
        if verbose_enabled {
            println!("# Tera output");
            println!("{}", rendered);
            println!("");
            println!("---");
            println!("");
        }
        input_bytes = rendered.into_bytes();
    }

    if to_variant == ToVariant::Systemd {
        let units: SystemdUnits = from_variant.deserialize_into(&input_bytes)?;

        if units.0.is_empty() {
            return Err(anyhow!(
                "Input for Systemd resulted in no units to process."
            ));
        }

        let processed_units = process_systemd_configs(units)?;

        if let Some(output_dir) = output {
            write_files(processed_units.0, &output_dir, utils::to_ini_string)?;
        } else {
            print_files(processed_units.0, utils::to_ini_string)?;
        }
    } else if to_variant == ToVariant::Quadlet {
        let _unit: serde_yaml::Value = from_variant.deserialize_into(&input_bytes)?;

        todo!();
    } else if let Some(output_file) = output {
        from_variant.serialize(input_bytes, |obj| {
            let buf = to_variant.to_buf(obj);
            std::fs::write(&output_file, buf).unwrap();
        });
    } else {
        from_variant.serialize(input_bytes, |obj| {
            let buf = to_variant.to_buf(obj);
            stdout().lock().write(&buf).unwrap();
        })
    }

    Ok(())
}

fn main() {
    let opts = Opts::parse();
    if let Err(e) = run(opts) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
