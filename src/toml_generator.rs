use serde_derive::Serialize;
use std::collections::BTreeMap;
use std::fs;
use toml;

#[derive(Serialize)]
pub struct TomlStructure {
    package: BTreeMap<String, String>,
    dependencies: BTreeMap<String, DependencyInfo>,
}

#[derive(Serialize)]
pub struct DependencyInfo {
    pub version: String,
    pub features: Option<Vec<String>>,
}

impl TomlStructure {
    pub fn new(name: String) -> Self {
        let mut package: BTreeMap<String, String> = BTreeMap::new();

        package.insert("name".into(), name);
        package.insert("version".into(), "0.1.0".into());
        package.insert("edition".into(), "2021".into());

        let mut dependencies: BTreeMap<String, DependencyInfo> = BTreeMap::new();
        dependencies.insert(
            "sea-orm".into(),
            DependencyInfo {
                version: "0.7.0".into(),
                features: Some(vec!["sqlx-sqlite".into(), "runtime-async-std-native-tls".into()]),
            },
        );
        dependencies.insert(
            "async-graphql".into(),
            DependencyInfo {
                version: "3.0.38".into(),
                features: Some(vec!["decimal".into(), "chrono".into()]),
            },
        );
        dependencies.insert(
            "async-graphql-poem".into(),
            DependencyInfo {
                version: "3.0.38".into(),
                features: None,
            },
        );
        dependencies.insert(
            "tokio".into(),
            DependencyInfo {
                version: "1.17.0".into(),
                features: Some(vec!["macros".into(), "rt-multi-thread".into()]),
            },
        );
        dependencies.insert(
            "poem".into(),
            DependencyInfo {
                version: "1.3.29".into(),
                features: None,
            },
        );

        Self {
            package,
            dependencies,
        }
    }
}

pub fn write_toml(path: &String, name: &String) {
    let data = TomlStructure::new(name.clone());

    fs::write(
        format!("{}/Cargo.toml", path),
        toml::to_string_pretty(&data).unwrap(),
    )
    .unwrap();
}
