//! Project links of Engine task objects. A task links to at most one project
//! (`project`); tags are not links (see agenda_core::mapping).
use super::{Engine, EngineError};
use agenda_core::mapping::{Link, PROJECT_LINK};
use serde_json::{json, Value};
use std::collections::HashMap;

impl Engine {
    /// The stored task object and its project link, read the way `load` reads them.
    pub(super) fn stored_task(&self, id: &str) -> Result<(Value, Vec<Link>), EngineError> {
        let object = self.rpc("get_object", json!({"id":id}))?;
        if object.is_null() {
            return Ok((object, Vec::new()));
        }
        let links = self.project_links()?.remove(id).unwrap_or_default();
        Ok((object, links))
    }

    /// Project links of every task, keyed by task id.
    pub(super) fn project_links(&self) -> Result<HashMap<String, Vec<Link>>, EngineError> {
        let list = self.rpc("list_object_links", json!({}))?;
        let list = list
            .as_array()
            .ok_or_else(|| Self::mapping_err("links not an array".into()))?;
        let mut by_task: HashMap<String, Vec<Link>> = HashMap::new();
        for link in list.iter().filter(|l| l["linkType"] == PROJECT_LINK) {
            let (Some(id), Some(source), Some(target)) = (
                link["id"].as_str(),
                link["sourceObjectId"].as_str(),
                link["targetObjectId"].as_str(),
            ) else {
                return Err(Self::mapping_err("link without id or endpoints".into()));
            };
            by_task.entry(source.to_owned()).or_default().push(Link {
                id: id.to_owned(),
                kind: PROJECT_LINK.to_owned(),
                target: target.to_owned(),
            });
        }
        Ok(by_task)
    }

    /// Replaces the task's project link: drops the stored one, then adds the new one.
    pub(super) fn relink_project(
        &self,
        task: &str,
        linked: Option<&Link>,
        project: Option<&str>,
    ) -> Result<(), EngineError> {
        if let Some(link) = linked {
            self.rpc("delete_object_link", json!({"id":link.id}))?;
        }
        let Some(project) = project else {
            return Ok(());
        };
        // Id from the endpoints, so a link added twice (or from two devices) is one row.
        let link = json!({
            "id": format!("lnk:{task}:{PROJECT_LINK}:{project}"),
            "sourceObjectId": task,
            "targetObjectId": project,
            "linkType": PROJECT_LINK,
            "createdAt": chrono::Utc::now().to_rfc3339(),
        });
        self.rpc("upsert_object_link", json!({"object_link":link}))?;
        Ok(())
    }
}
