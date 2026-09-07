use std::collections::HashMap;

use gitlab::{Gitlab, api::common::AccessLevel};

use super::{api, config};

pub(crate) fn create_group_and_projects(
    client: &Gitlab,
    config_group: &config::ConfigGroup,
    parent_gitlab_group_id: Option<u64>,
    group_hashmap: &mut HashMap<u64, u64>,
    project_hashmap: &mut HashMap<u64, u64>,
) {
    // Create this group itself
    let group_info = api::create_group(client, parent_gitlab_group_id, &config_group.name);
    match group_info {
        Ok(group_info) => {
            // Map our custom config ID -> GitLab group ID
            group_hashmap.insert(config_group.id, group_info.id);

            let current_gitlab_group_id = Some(group_info.id);

            // Create projects belonging to this group
            if let Some(projects) = &config_group.projects {
                for project in projects {
                    let project_info = api::create_project(
                        client,
                        current_gitlab_group_id,
                        &project.name,
                        project.import_url.as_deref(),
                    );

                    match project_info {
                        Ok(project_info) => {
                            println!(
                                "Created project '{}' with ID: {}",
                                project.name, project_info.id
                            );
                            project_hashmap.insert(project.id, project_info.id);
                        }
                        Err(e) => {
                            eprintln!("Failed to create project '{}': {}", project.name, e);
                        }
                    }
                }
            }

            // Recursively create subgroups
            if let Some(subgroups) = &config_group.subgroups {
                for subgroup in subgroups {
                    create_group_and_projects(
                        client,
                        subgroup,
                        current_gitlab_group_id,
                        group_hashmap,
                        project_hashmap,
                    );
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to create group '{}': {}", config_group.name, e);
        }
    }
}

pub(crate) fn convert_access_level(access_level: u64) -> AccessLevel {
    match access_level {
        50 => AccessLevel::Owner,
        40 => AccessLevel::Maintainer,
        30 => AccessLevel::Developer,
        20 => AccessLevel::Reporter,
        10 => AccessLevel::Guest,
        _ => AccessLevel::Guest,
    }
}
