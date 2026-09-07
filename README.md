# GL-Forge

**glforge** is a command-line tool written in Rust for bulk-provisioning GitLab: nested groups, template repositories, and user permissions. It generates a safe-to-review GitLab structure first, then creates everything through the GitLab API, either from any CSV export (Canvas, other LMS, or a hand-made spreadsheet) or directly from JSON/CSV files you craft yourself.

## Features

- **Two-step workflow**: Generate and inspect your GitLab structure before committing any changes to the API.
- **Any CSV input**: The `build` command is CSV-agnostic: configure which columns hold the username and group id, so any export works.
- **Standalone GitLab provisioning**: Skip the CSV step entirely and feed the tool a JSON hierarchy and a CSV user list to build GitLab infrastructure directly.
- **Custom hierarchies**: Map flat CSV groups into nested GitLab subgroups (e.g., by year, cohort, or module).
- **Template projects**: Automatically scaffold repositories inside student groups with optional starting code (`import_url`).
- **Bulk user management**: Look up users by their institutional username (e.g., student number) and assign the correct GitLab access level (Owner, Maintainer, Developer, etc.).
- **Missing-user check**: Verify which users in your access list do not yet exist in GitLab.

## Table of Contents

- [Installation](#installation)
- [CLI Commands](#cli-commands)
  - [`build`](#1-the-build-command)
  - [`gitlab`](#2-the-gitlab-command)
  - [`missing-user`](#3-the-missing-user-command)
- [Standalone GitLab Provisioning](#standalone-gitlab-provisioning)
- [Configuration Guide](#configuration-guide-csv-processing)
  - [GitLab access levels](#gitlab-access-levels)
  - [The `config.json` file](#building-your-configjson)
  - [The input CSV](#understanding-the-input-csv)
- [Project Structure](#project-structure)
- [Security & Tokens](#security--tokens)
- [License](#license)

## Installation

Ensure you have [Rust and Cargo](https://rustup.rs/) installed, then clone and build:

```bash
git clone <repository-url>
cd GL-Forge
cargo build --release
```

The compiled binary will be available at `./target/release/glforge`.

> **Tip**: Run `glforge --help` at any time to see the full command reference.

## Workflow Overview

Using the tool is a two-step process:

1. **`build` command**: Reads an input CSV (Canvas export or any other spreadsheet) and your `config.json` to generate a nested JSON hierarchy and a user-access CSV. Nothing is changed in GitLab yet.
2. **`gitlab` command**: Reads the generated files and creates the groups, projects, and user permissions through the GitLab API.

_If you are provisioning GitLab directly from hand-crafted files, you only need Step 2._

## CLI Commands

### 1. The `build` Command

Converts an input CSV export (Canvas or any other spreadsheet) into the nested JSON hierarchy and user-access list required for GitLab syncing. The columns to read are configured in your `config.json`; see [the CSV section](#understanding-the-input-csv).

```bash
glforge build <CONFIG_FILE> <INPUT_CSV> <OUTPUT_USER_CSV> <OUTPUT_JSON>
```

Example:

```bash
glforge build config.json students_export.csv user_access.csv group_structure.json
```

### 2. The `gitlab` Command

Reads the prepared JSON and CSV files and executes the required API calls against your GitLab instance.

```bash
glforge gitlab <TOKEN> <URL> <STRUCTURE_JSON> <USERS_CSV>
```

Example:

```bash
glforge gitlab glpat-your-token-here gitlab.utwente.nl group_structure.json user_access.csv
```

### 3. The `missing-user` Command

Checks which usernames in your access CSV do **not** have a matching GitLab account, so you can resolve them before syncing.

```bash
glforge missing-user <TOKEN> <URL> <USERS_CSV>
```

- See [Standalone GitLab Provisioning](#standalone-gitlab-provisioning) for the exact `group_structure.json` and `users.csv` formats.
- Preconfigured example files are available in [`gitlab_config_example/`](gitlab_config_example/).

---

## Standalone GitLab Provisioning

If you use a different LMS, custom spreadsheets, or just want to quickly build a GitLab folder structure, skip the `build` command entirely and hand-craft the two inputs the `gitlab` command expects: `group_structure.json` and `users.csv`.

### 1. The `group_structure.json` File

This file is the blueprint for your GitLab folder structure; it defines the nested hierarchy of groups, subgroups, and projects to create.

```json
{
  "parent_group_id": null,
  "name": "Root_Group2",
  "id": 100,
  "projects": [],
  "subgroups": [
    {
      "name": "Student_Group_A",
      "id": 300,
      "projects": [
        {
          "name": "Main",
          "id": 3001,
          "import_url": "https://github.com/rust-lang/rustlings.git"
        },
        { "name": "Template", "id": 3002, "import_url": null }
      ],
      "subgroups": null
    }
  ]
}
```

#### Group-Level Fields

- `parent_group_id`: To create this entire tree inside an existing GitLab group, put that group's actual GitLab ID here (visible on the group's homepage). Leave `null` to create the root group at the top level of your GitLab instance.
- `name`: The name of the group or subgroup as it will appear in GitLab (e.g., `"Student_Group_A"`).
- `id`: **Important.** An _internal mapping ID_ that you invent. It does not exist in GitLab yet; it is the "glue" between this JSON and your `users.csv`. Assigning a user to `300` in the CSV places them in the group with `"id": 300`.
- `subgroups`: Allows infinite nesting. Each subgroup uses the exact same format (and can contain its own `subgroups` and `projects`). Use `null` or `[]` if there are no subgroups.

#### Project-Level Fields (`projects` array)

Projects are the actual Git repositories where code lives.

- `name`: The name of the repository (e.g., `"Main"`).
- `id`: Like the group `id`, an internal mapping ID. Use this in `users.csv` to grant a user access to a specific project rather than the whole group.
- `import_url`: A `.git` URL to seed the repository with starting code; GitLab clones it automatically on creation. Use `null` for a blank repository.
  - **Public repositories**: use a plain `.git` URL, e.g. `https://github.com/rust-lang/rustlings.git`.
  - **Private repositories**: the URL must be a git link with embedded credentials, e.g. `https://oauth2:<token>@gitlab.example.com/group/repo.git`. Replace `<token>` with a GitLab Personal Access Token or use a GitLab OAuth2 token as shown below.

### 2. The `users.csv` File

Tells the tool which GitLab users belong to which internal `id` from your JSON.

- `username`: The institutional username (e.g., `s1`) used to look up the GitLab account.
- `name`: The user's display name (used for reference only).
- `access_id`: Must match an `id` from a group or project in your `group_structure.json`.
- `access_level`: The standard GitLab integer for permissions (e.g., `50` = Owner, `40` = Maintainer, `30` = Developer).

```csv
username,name,access_id,access_level
s1,Jacco te Poel,100,50
s2,John Doe,300,40
```

Once these files are ready, run the `gitlab` command to build the structure.

- Working examples live in [`gitlab_config_example/`](gitlab_config_example/).

---

## Configuration Guide (CSV Processing)

### GitLab Access Levels

The tool maps integers to GitLab roles. An unrecognized number defaults to `Guest (10)`.

| Integer | GitLab Role | Description                                                                                                      |
| ------- | ----------- | ---------------------------------------------------------------------------------------------------------------- |
| **50**  | Owner       | Full administrative access to the group/project.                                                                 |
| **40**  | Maintainer  | Can manage repository settings, push to protected branches, and manage users. _(Recommended for group leaders)._ |
| **30**  | Developer   | Can push code and create merge requests. _(Recommended for standard students)._                                  |
| **20**  | Reporter    | Read-only access to code, can manage issues.                                                                     |
| **10**  | Guest       | Minimal access; can only see public data or comment on issues.                                                   |

### Building your `config.json`

This file tells the `build` command how to transform a flat CSV into a nested GitLab hierarchy, separating **settings** from **hierarchy**. The settings include column mappings so any CSV layout works.

```json
{
  "settings": {
    "root_group_id": null,
    "default_access_level": 40,
    "replace_spaces_with_underscore": true,
    "username_column": "login_id",
    "group_id_column": "canvas_group_id",
    "group_name_column": "group_name",
    "name_column": "name",
    "project_templates": [
      { "name": "Main", "import_url": null, "id_offset": 1 }
    ]
  },
  "hierarchy": {
    "name": "Example_Course",
    "internal_id": 100,
    "subgroups": [
      {
        "name": "2026",
        "internal_id": 150,
        "subgroups": [
          {
            "name": "Pi1",
            "internal_id": 200,
            "group_ids": [221493, 221494]
          }
        ]
      }
    ]
  }
}
```

#### Settings Properties

The four column mapping fields (`username_column`, `group_id_column`, `group_name_column`, and `name_column`) are **required** — they must match the actual column names of your CSV.

- `root_group_id`: The GitLab group ID to create the course structure inside. Leave `null` to create it at the top level of your GitLab instance.
- `default_access_level`: The GitLab role integer (e.g., `40` for Maintainer) assigned to every user parsed from the CSV.
- `replace_spaces_with_underscore`: `true`/`false`; when enabled, automatically replaces spaces with underscores in generated group names.
- `username_column` (required): The CSV column containing the GitLab username (e.g., a student number). The values must match existing GitLab usernames, since the tool looks users up by this field.
- `group_id_column` (required): The CSV column containing the group id. These values are matched against `internal_id`/`group_ids` in your hierarchy to route users to the right GitLab subgroup.
- `group_name_column` (required): The CSV column containing the human-readable group name. This becomes the GitLab group name. If the column is missing or empty, the group id string is used. You may match groups by name instead of id using `group_names`.
- `name_column` (required): The CSV column containing the user's display name (used for reference only). Falls back to the username if the column is missing or empty.
- `project_templates`: A list of repositories to generate inside the lowest-level student groups:
  - `name`: The name of the repository.
  - `import_url`: (Optional) A `.git` URL to clone starting code from. Set to `null` for an empty repo. For private repositories this must be a git link with embedded credentials, e.g. `https://oauth2:<token>@gitlab.example.com/group/repo.git`.
  - `id_offset`: An integer added to the group's ID so generated project IDs stay unique.

#### Hierarchy Properties

Defines your custom folder tree recursively, from the root course down to its sub-units.

- `name`: The name of the group/subgroup in GitLab.
- `internal_id`: An arbitrary, unique integer used internally by the tool to track this node.
- `group_ids`: An array of integers matched against the values in your CSV's group id column. Users in those groups are placed into this GitLab subgroup. You may also match by group name using `group_names`.
- `group_names`: (Optional) An array of group names matched against your CSV's group name column, as an alternative to `group_ids`.

A complete example is available in [`csv_config_example/config.json`](csv_config_example/config.json).

### Understanding the input CSV

The tool does **not** assume a fixed CSV layout; the `username_column`, `group_id_column`, `group_name_column`, and `name_column` settings tell it which columns to read (column names are matched case-insensitively). All four must be set explicitly. This means exports from Canvas or any other LMS/spreadsheet work, as long as you map the right columns.

For example, here is how a standard Canvas course export (created via **People > Groups**, then export the group data) maps to the settings. While Canvas exports many columns, the tool primarily uses:

| Column Name       | Config Setting      | Purpose in this Tool                                                                                                                  |
| ----------------- | ------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `name`            | `name_column`       | The user's full name (used for logging and verification).                                                                             |
| `login_id`        | `username_column`   | **Crucial.** The user's institutional username (e.g., `s2978881`), used to look up their existing GitLab account.                     |
| `canvas_group_id` | `group_id_column`   | **Crucial.** The unique identifier for the user's assigned group. Place these numbers in the `group_ids` array in your `config.json`. |
| `group_name`      | `group_name_column` | The human-readable name of the group (becomes the GitLab group name; can also be matched via `group_names`).                          |

_(Columns such as `canvas_user_id`, `user_id`, and `sections` are ignored unless you map them.)_

> **Important:** The username column must contain **GitLab usernames**. The tool matches every user by this value against existing GitLab accounts, so values must match exactly (with a student-number-based GitLab instance, the `login_id` is the student number). Run `glforge missing-user <TOKEN> <URL> <USERS_CSV>` after the `build` command to spot accounts that are missing.

A sample export is available at [`csv_config_example/sample_export.csv`](csv_config_example/sample_export.csv).

---

## Project Structure

```
src/
├── main.rs              # CLI entry point and command dispatch
├── build/
│   └── converter.rs     # CSV -> JSON hierarchy + user CSV
└── gitlab/
    ├── config.rs        # Parsing of group_structure.json and users.csv
    ├── api.rs           # Low-level GitLab API calls
    └── client.rs        # Recursive group/project creation, access levels
```

## Security & Tokens

- The `gitlab` and `missing-user` commands require a **Personal Access Token** (PAT) with the `api` scope.
- Never commit GitLab tokens or sensitive user data to version control. The included `.gitignore` already excludes `.env`.

## Disclaimer

GL-Forge is an independent open-source project and is not affiliated with, maintained by, sponsored by, or endorsed by GitLab Inc. The GitLab name and logo are trademarks of GitLab Inc.

## License

This project is licensed under the [MIT License](https://opensource.org/licenses/MIT).
