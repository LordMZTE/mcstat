use clap::{App, YamlLoader};
use lazy_static::lazy_static;
use yaml_rust::Yaml;

lazy_static! {
    static ref YAML: Yaml = YamlLoader::load_from_str(include_str!("args.yml"))
        .unwrap()
        .pop()
        .unwrap();
}

pub fn get_app() -> App<'static, 'static> {
    App::from_yaml(&YAML)
}
