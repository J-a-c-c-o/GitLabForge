use serde::Deserialize;

// --- CSV Record Structures ---
#[derive(Deserialize, Debug)]
pub struct CsvRecord {
    pub(crate) username: String,
    #[allow(dead_code)]
    pub(crate) name: String,
    pub(crate) access_id: u64,
    pub(crate) access_level: u64,
}

// --- JSON Configuration Structures ---
#[derive(Deserialize, Debug)]
pub struct ProvisionConfig {
    pub(crate) parent_group_id: Option<u64>,
    pub(crate) name: String,
    pub(crate) id: u64,
    pub(crate) subgroups: Vec<ConfigGroup>,
    pub(crate) projects: Vec<ConfigProject>,
}

impl From<ProvisionConfig> for ConfigGroup {
    fn from(config: ProvisionConfig) -> Self {
        ConfigGroup {
            name: config.name,
            id: config.id,
            projects: Some(config.projects),
            subgroups: Some(config.subgroups),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct ConfigGroup {
    pub(crate) name: String,
    pub(crate) id: u64,
    pub(crate) projects: Option<Vec<ConfigProject>>,
    pub(crate) subgroups: Option<Vec<ConfigGroup>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ConfigProject {
    pub(crate) name: String,
    pub(crate) id: u64,
    pub(crate) import_url: Option<String>,
}

pub fn load_config_from_file(path: &str) -> Result<ProvisionConfig, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let config: ProvisionConfig = serde_json::from_reader(file)?;
    Ok(config)
}

pub fn load_users_from_csv(path: &str) -> Result<Vec<CsvRecord>, Box<dyn std::error::Error>> {
    let mut rdr = csv::Reader::from_path(path)?;
    let mut users = Vec::new();
    for result in rdr.deserialize() {
        let record: CsvRecord = result?;
        users.push(record);
    }
    Ok(users)
}
