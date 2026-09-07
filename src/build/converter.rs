use csv::StringRecord;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::error::Error;
use std::fs::File;

/// Template defining projects attached to each student group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTemplate {
    pub name: String,
    pub import_url: Option<String>,
    pub id_offset: u64,
}

/// Global settings for the generator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Default access level for all users in the generated CSV
    pub default_access_level: u32,

    /// Optional root group id to attach all generated groups to.
    pub root_group_id: Option<u64>,

    /// Whether to replace spaces in group names with underscores when generating the JSON structure.
    #[serde(default)]
    pub replace_spaces_with_underscore: bool,

    /// List of project templates to attach to each student group
    pub project_templates: Vec<ProjectTemplate>,

    /// CSV column containing the username (e.g. student number).
    pub username_column: String,

    /// CSV column containing the group id.
    pub group_id_column: String,

    /// CSV column containing the human-readable group name.
    pub group_name_column: String,

    /// CSV column containing the user's display name.
    pub name_column: String,
}

/// Find the index of a column by header name (case-insensitive).
fn find_column(headers: &StringRecord, column: &str) -> Option<usize> {
    headers.iter().position(|h| h.eq_ignore_ascii_case(column))
}

/// Resolve a configured column, erroring with a helpful message when it is missing.
fn resolve_column(headers: &StringRecord, column: &str) -> Result<usize, Box<dyn Error>> {
    find_column(headers, column).ok_or_else(|| {
        format!(
            "Column '{column}' not found in CSV headers. Configure the correct column name in the 'settings' section of your config.json."
        )
        .into()
    })
}

/// Recursive config node for arbitrary nesting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchyNode {
    pub name: String,
    pub internal_id: u64,

    #[serde(default)]
    pub group_ids: Vec<u64>,

    #[serde(default)]
    pub group_names: Vec<String>,

    #[serde(default)]
    pub subgroups: Vec<HierarchyNode>,
}

/// Root config structure for the generator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicStructureConfig {
    pub settings: Settings,
    pub hierarchy: HierarchyNode,
}

/// Output CSV row for membership permissions
#[derive(Debug, Serialize)]
struct UserAccessRecord {
    username: String,
    name: String,
    access_id: u64,
    access_level: u32,
}

/// Sub-project schema inside output JSON
#[derive(Debug, Serialize, Clone)]
struct Project {
    name: String,
    id: u64,
    import_url: Option<String>,
}

/// Output Group node schema inside output JSON
#[derive(Debug, Serialize, Clone)]
struct Group {
    parent_group_id: Option<u64>,
    name: String,
    id: u64,
    projects: Vec<Project>,
    subgroups: Option<Vec<Group>>,
}

pub fn process_csv(
    input_path: &str,
    config_path: &str,
    user_csv_output: &str,
    json_output: &str,
) -> Result<(), Box<dyn Error>> {
    println!("Reading CSV: {}", input_path);
    let mut reader = csv::Reader::from_path(input_path)?;

    let config_file = File::open(config_path)?;
    let config: DynamicStructureConfig = serde_json::from_reader(config_file)?;

    let headers = reader.headers()?.clone();
    let username_col = resolve_column(&headers, &config.settings.username_column)?;
    let group_id_col = resolve_column(&headers, &config.settings.group_id_column)?;
    let group_name_col = resolve_column(&headers, &config.settings.group_name_column)?;
    let name_col = find_column(&headers, &config.settings.name_column);

    let mut user_records: Vec<UserAccessRecord> = Vec::new();
    let mut student_groups: BTreeMap<u64, String> = BTreeMap::new();

    for result in reader.records() {
        let record = match result {
            Ok(rec) => rec,
            Err(e) => {
                eprintln!("Error parsing CSV record: {}", e);
                continue;
            }
        };

        let Some(username) = record
            .get(username_col)
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            eprintln!(
                "Missing username ({}) in record: {:?}",
                config.settings.username_column, record
            );
            continue;
        };

        let Some(group_id) = record
            .get(group_id_col)
            .and_then(|v| v.trim().parse::<u64>().ok())
        else {
            eprintln!(
                "Missing or invalid group id ({}) in record: {:?}",
                config.settings.group_id_column, record
            );
            continue;
        };

        let group_name = record
            .get(group_name_col)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| group_id.to_string());

        let name = name_col
            .and_then(|col| record.get(col))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| username.to_string());

        let formatted_gname = if config.settings.replace_spaces_with_underscore {
            group_name.trim().replace(' ', "_")
        } else {
            group_name.trim().to_string()
        };

        user_records.push(UserAccessRecord {
            username: username.to_string(),
            name,
            access_id: group_id,
            access_level: config.settings.default_access_level,
        });

        student_groups.insert(group_id, formatted_gname);
    }

    // Write output CSV
    let mut csv_writer = csv::Writer::from_path(user_csv_output)?;
    for record in &user_records {
        csv_writer.serialize(record)?;
    }
    csv_writer.flush()?;
    println!("Generated user permissions CSV: {}", user_csv_output);

    let root_group = build_group_node(
        &config.hierarchy,
        config.settings.root_group_id,
        &student_groups,
        &config.settings.project_templates,
        config.settings.replace_spaces_with_underscore,
    );

    // Write output JSON
    let json_file = File::create(json_output)?;
    serde_json::to_writer_pretty(json_file, &root_group)?;
    println!("Generated structure JSON: {}", json_output);

    Ok(())
}

/// Helper function to build intermediate group nodes recursively
fn build_group_node(
    node_cfg: &HierarchyNode,
    parent_id: Option<u64>,
    student_groups: &BTreeMap<u64, String>,
    templates: &[ProjectTemplate],
    format_spaces: bool,
) -> Group {
    let mut child_nodes: Vec<Group> = Vec::new();

    // 1. Recurse down nested intermediate subgroups
    for sub_cfg in &node_cfg.subgroups {
        let child = build_group_node(
            sub_cfg,
            Some(node_cfg.internal_id),
            student_groups,
            templates,
            format_spaces,
        );
        child_nodes.push(child);
    }

    // 2. Attach matched student groups
    let target_ids: HashSet<u64> = node_cfg.group_ids.iter().cloned().collect();
    let target_names: HashSet<String> = node_cfg.group_names.iter().cloned().collect();

    for (&gid, gname) in student_groups {
        if target_ids.contains(&gid) || target_names.contains(gname) {
            let projects = templates
                .iter()
                .map(|tmpl| Project {
                    name: tmpl.name.clone(),
                    id: gid * 10 + tmpl.id_offset,
                    import_url: tmpl.import_url.clone(),
                })
                .collect();

            child_nodes.push(Group {
                parent_group_id: Some(node_cfg.internal_id),
                name: gname.clone(),
                id: gid,
                projects,
                subgroups: None,
            });
        }
    }

    let group_name = if format_spaces {
        node_cfg.name.trim().replace(' ', "_")
    } else {
        node_cfg.name.trim().to_string()
    };

    Group {
        parent_group_id: parent_id,
        name: group_name,
        id: node_cfg.internal_id,
        projects: vec![],
        subgroups: if child_nodes.is_empty() {
            None
        } else {
            Some(child_nodes)
        },
    }
}
