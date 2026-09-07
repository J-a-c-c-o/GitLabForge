use gitlab::api::common::AccessLevel;
use gitlab::api::groups::members::AddGroupMember;
use gitlab::api::{Query, groups, projects, users};
use gitlab::{Gitlab, api};
use serde::Deserialize;

use anyhow::Result;

#[derive(Deserialize, Debug)]
pub struct GroupInfo {
    pub(crate) id: u64,
}

#[derive(Deserialize, Debug)]
pub struct ProjectInfo {
    pub(crate) id: u64,
}

#[derive(Deserialize, Debug)]
pub struct UserInfo {
    pub(crate) id: u64,
}

pub(crate) fn create_project(
    client: &Gitlab,
    group: Option<u64>,
    project_name: &str,
    import_url: Option<&str>,
) -> Result<ProjectInfo> {
    let project_endpoint = if let Some(url) = import_url {
        if let Some(group) = group {
            projects::CreateProject::builder()
                .name(project_name)
                .path(project_name)
                .namespace_id(group)
                .import_url(url)
                .visibility(api::common::VisibilityLevel::Private)
                .build()
        } else {
            projects::CreateProject::builder()
                .name(project_name)
                .path(project_name)
                .import_url(url)
                .visibility(api::common::VisibilityLevel::Private)
                .build()
        }
    } else {
        if let Some(group) = group {
            projects::CreateProject::builder()
                .name(project_name)
                .path(project_name)
                .namespace_id(group)
                .visibility(api::common::VisibilityLevel::Private)
                .build()
        } else {
            projects::CreateProject::builder()
                .name(project_name)
                .path(project_name)
                .visibility(api::common::VisibilityLevel::Private)
                .build()
        }
    }?;

    let project: ProjectInfo = project_endpoint.query(client)?;
    Ok(project)
}

pub(crate) fn create_group(
    client: &Gitlab,
    group_id: Option<u64>,
    group_name: &str,
) -> Result<GroupInfo> {
    let group_endpoint = if let Some(id) = group_id {
        groups::CreateGroup::builder()
            .name(group_name)
            .path(group_name)
            .parent_id(id)
            .visibility(api::common::VisibilityLevel::Private)
            .build()
    } else {
        groups::CreateGroup::builder()
            .name(group_name)
            .path(group_name)
            .visibility(api::common::VisibilityLevel::Private)
            .build()
    }?;

    let group: GroupInfo = group_endpoint.query(client)?;

    Ok(group)
}

pub(crate) fn find_user_by_username(client: &Gitlab, username: &str) -> Result<Option<u64>> {
    let endpoint = users::Users::builder().username(username).build()?;

    let matching_users: Vec<UserInfo> = endpoint.query(client)?;

    Ok(matching_users.first().map(|user| user.id))
}

pub(crate) fn add_user_to_group(
    client: &Gitlab,
    group_id: u64,
    user_id: u64,
    access_level: AccessLevel,
) -> Result<()> {
    println!(
        "Adding user (ID: {}) to group (ID: {}) with access level {:?}...",
        user_id, group_id, access_level
    );

    let endpoint = AddGroupMember::builder()
        .group(group_id) // The group they are joining
        .user(user_id) // The user being added
        .access_level(access_level)
        .build()?;

    api::ignore(endpoint).query(client)?;

    println!("Successfully added the user to the group!");

    Ok(())
}

pub(crate) fn add_user_to_project(
    client: &Gitlab,
    project_id: u64,
    user_id: u64,
    access_level: AccessLevel,
) -> Result<()> {
    println!(
        "Adding user (ID: {}) to project (ID: {}) with access level {:?}...",
        user_id, project_id, access_level
    );

    let endpoint = projects::members::AddProjectMember::builder()
        .project(project_id) // The project they are joining
        .user(user_id) // The user being added
        .access_level(access_level)
        .build()?;

    api::ignore(endpoint).query(client)?;

    println!("Successfully added the user to the project!");
    Ok(())
}
