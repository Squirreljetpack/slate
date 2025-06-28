use anyhow::Result;
use slate::quadlet::{process_compose, ComposeFile};
use slate::utils::enter_test_dir;
use std::{fs::File};

use std::sync::Once;

static INIT: Once = Once::new();

fn setup_test_env() {
    INIT.call_once(|| {
        std::env::set_var("RUST_LOG", "debug");
        env_logger::init();
        std::env::set_var("SLATER_AUTO", "true");
        std::env::set_var("NGINX_HOST", "localhost");
    });
}

// pub fn replace_cwd(file: &mut ComposeFile) -> anyhow::Result<()> {
//     let mut cwd = std::env::current_dir()?.to_string_lossy().to_string();
//     if !cwd.ends_with('/') {
//         cwd.push('/');
//     }
//     let slater_prefix = "/slater/".to_string();

//     for service in file.services.values_mut() {
//         if let Some(service_map) = service.as_mapping_mut() {
//             if let Some(volumes_val) = service_map.get_mut(&Value::String("volumes".to_string())) {
//                 if let Some(volumes) = volumes_val.as_sequence_mut() {
//                     for volume in volumes.iter_mut() {
//                         if let Some(volume_str) = volume.as_str() {
//                             if volume_str.starts_with(&cwd) {
//                                 let replaced = volume_str.replacen(&cwd, &slater_prefix, 1);
//                                 *volume = Value::String(replaced);
//                             }
//                         }
//                     }
//                 }
//             }
//         }
//     }

//     Ok(())
// }


#[test]
fn test_process_compose() -> Result<()> {
    setup_test_env();
    let file = File::open("tests/fixtures/compose.yaml")?;
    let file: ComposeFile = serde_yaml::from_reader(file)?;

    enter_test_dir();
    let file = process_compose(file, None)?;

    insta::assert_yaml_snapshot!(file);
    Ok(())
}