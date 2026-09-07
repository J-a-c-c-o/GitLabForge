use ::gitlab::Gitlab;
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::collections::HashMap;

mod build;
mod gitlab;

use gitlab::config::CsvRecord;

/// CLI tool for provisioning GitLab groups, projects, and users from CSV/JSON exports
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert an input CSV export to nested JSON hierarchy and Access CSV
    Build {
        /// Path to the JSON configuration file
        ///
        /// Example config.json:
        /// {
        ///   "settings": {
        ///     "root_group_id": null,
        ///     "default_access_level": 40,
        ///     "replace_spaces_with_underscore": true,
        ///     "username_column": "login_id",
        ///     "group_id_column": "canvas_group_id",
        ///     "group_name_column": "group_name",
        ///     "name_column": "name",
        ///     "project_templates": [
        ///       { "name": "Main", "import_url": null, "id_offset": 1 }
        ///     ]
        ///   },
        ///   "hierarchy": {
        ///     "name": "Example_Course",
        ///     "internal_id": 100,
        ///     "subgroups": [
        ///       {
        ///         "name": "2026",
        ///         "internal_id": 150,
        ///         "subgroups": [
        ///           {
        ///             "name": "Pi1",
        ///             "internal_id": 200,
        ///             "group_ids": [221493, 221494]
        ///           }
        ///         ]
        ///       }
        ///     ]
        ///   }
        /// }
        #[arg(verbatim_doc_comment)]
        config_file: String,

        /// Path to the input CSV file
        ///
        /// Example input.csv:
        /// name,user_id,login_id,group_name,group_id
        /// Jacco te Poel,3343707,s1,Project Groups 38,221494
        /// John Doe,3410749,s2,Project Groups 10,221493
        #[arg(verbatim_doc_comment)]
        input_csv: String,

        /// Path for the output user access CSV
        output_user_csv: String,

        /// Path for the output group structure JSON
        output_json: String,
    },

    /// Sync the generated group structure and users to GitLab
    Gitlab {
        /// GitLab personal access token
        token: String,

        /// GitLab URL
        url: String,

        /// Path to the group structure JSON
        ///
        /// Example group_structure.json:
        /// {
        ///   "parent_group_id": null,
        ///   "name": "Root_Group2",
        ///   "id": 100,
        ///   "projects": [],
        ///   "subgroups": [
        ///     {
        ///       "name": "Student_Group_A",
        ///       "id": 300,
        ///       "projects": [
        ///         { "name": "Main", "id": 3001, "import_url": "https://..." },
        ///         { "name": "Template", "id": 3002, "import_url": null }
        ///       ],
        ///       "subgroups": null
        ///     }
        ///   ]
        /// }
        #[arg(verbatim_doc_comment)]
        structure_json: String,

        /// Path to the user access CSV
        ///
        /// Example users.csv:
        /// username,name,access_id,access_level
        /// s1,Jacco te Poel,100,50
        /// s2,John Doe,300,40
        #[arg(verbatim_doc_comment)]
        users_csv: String,
    },

    /// Check which users from the CSV do not exist in GitLab
    MissingUser {
        /// GitLab personal access token
        token: String,

        /// GitLab URL
        url: String,

        /// Path to the user access CSV
        ///
        /// Example users.csv:
        /// username,name,access_id,access_level
        /// s1,Jacco te Poel,100,50
        /// s2,John Doe,300,40
        #[arg(verbatim_doc_comment)]
        users_csv: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Build {
            config_file,
            input_csv,
            output_user_csv,
            output_json,
        } => {
            println!("Running CSV Converter...");
            match build::converter::process_csv(
                input_csv,
                config_file,
                output_user_csv,
                output_json,
            ) {
                Ok(_) => {
                    println!("\nSuccess!");
                    println!("  -> Access Permissions CSV : {}", output_user_csv);
                    println!("  -> Nested Hierarchy JSON  : {}", output_json);
                }
                Err(e) => {
                    eprintln!("\nError executing converter: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Gitlab {
            token,
            url,
            structure_json,
            users_csv,
        } => {
            println!("Running GitLab Sync...");

            let client = connect_gitlab(token, url);
            let config = match gitlab::config::load_config_from_file(structure_json) {
                Ok(cfg) => cfg,
                Err(e) => {
                    eprintln!("Failed to load configuration: {}", e);
                    std::process::exit(1);
                }
            };
            let users = load_users(users_csv);

            let mut group_hashmap: HashMap<u64, u64> = HashMap::new();
            let mut project_hashmap: HashMap<u64, u64> = HashMap::new();

            let parent_id = config.parent_group_id;
            gitlab::client::create_group_and_projects(
                &client,
                &config.into(),
                parent_id,
                &mut group_hashmap,
                &mut project_hashmap,
            );

            println!("Successfully created groups and projects!");
            println!("Group ID mapping: {:?}", group_hashmap);
            println!("Project ID mapping: {:?}", project_hashmap);

            for user in users {
                if let Some(user_id) = lookup_user(&client, &user.username) {
                    assign_user(&client, &user, user_id, &group_hashmap, &project_hashmap);
                }
            }
            println!("Finished assigning users!");
        }

        Commands::MissingUser {
            token,
            url,
            users_csv,
        } => {
            println!("Checking for missing users in GitLab...");

            let client = connect_gitlab(token, url);
            let users = load_users(users_csv);

            for user in users {
                lookup_user(&client, &user.username);
            }
        }
    }

    Ok(())
}

fn connect_gitlab(token: &str, url: &str) -> Gitlab {
    match Gitlab::new(url, token) {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Failed to connect to GitLab: {}", e);
            std::process::exit(1);
        }
    }
}

fn load_users(users_csv: &str) -> Vec<CsvRecord> {
    match gitlab::config::load_users_from_csv(users_csv) {
        Ok(users) => users,
        Err(e) => {
            eprintln!("Failed to load users from CSV: {}", e);
            std::process::exit(1);
        }
    }
}

fn lookup_user(client: &Gitlab, username: &str) -> Option<u64> {
    match gitlab::api::find_user_by_username(client, username) {
        Ok(Some(user_id)) => Some(user_id),
        Ok(None) => {
            eprintln!("User not found in GitLab: {}", username);
            None
        }
        Err(e) => {
            eprintln!(
                "Error occurred while searching for user {}: {}",
                username, e
            );
            None
        }
    }
}

fn assign_user(
    client: &Gitlab,
    user: &CsvRecord,
    user_id: u64,
    group_hashmap: &HashMap<u64, u64>,
    project_hashmap: &HashMap<u64, u64>,
) {
    let access_level = gitlab::client::convert_access_level(user.access_level);

    if let Some(&gid) = group_hashmap.get(&user.access_id) {
        let _ = gitlab::api::add_user_to_group(client, gid, user_id, access_level);
    } else if let Some(&pid) = project_hashmap.get(&user.access_id) {
        let _ = gitlab::api::add_user_to_project(client, pid, user_id, access_level);
    } else {
        eprintln!(
            "No matching group or project found for access ID: {}",
            user.access_id
        );
    }
}
