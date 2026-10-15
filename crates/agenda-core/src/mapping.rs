//! core.task 0.1.0 mapping (KOS-397): `read` turns an Engine object and its
//! links into a `Todo`; `write_at` patches an Engine object from a `Todo`.
mod read;
mod write;

pub use read::read;
pub use write::write_at;

/// core.task 0.1.0: the Engine's task contract. The project is an object link
/// (`project`, at most one), not a prop: `read` takes the links the caller
/// loaded, and `write_at` leaves link changes to the caller. Tags and other
/// Agenda-only fields stay in `extensions`.
pub const TASK_TYPE: &str = "core.task";
pub const TASK_VERSION: &str = "0.1.0";
pub const PROJECT_LINK: &str = "project";

/// An outbound object link: `kind` is the link type, `target` the object id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub id: String,
    pub kind: String,
    pub target: String,
}

/// Links carried on an object payload, in the data-API spelling
/// (`target_object_id`, `link_type`) or the camel-case one.
pub fn links_in_object(object: &serde_json::Value) -> Vec<Link> {
    object["links"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|link| {
            let target = link
                .get("target_object_id")
                .or_else(|| link.get("targetObjectId"))?
                .as_str()?;
            let kind = link
                .get("link_type")
                .or_else(|| link.get("linkType"))?
                .as_str()?;
            Some(Link {
                id: link["id"].as_str().unwrap_or_default().to_owned(),
                kind: kind.to_owned(),
                target: target.to_owned(),
            })
        })
        .collect()
}
