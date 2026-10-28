use std::sync::Arc;

use futures_lite::future::yield_now;
use ordered_float::OrderedFloat;
use warp_core::ui::appearance::Appearance;
use warpui::fonts::FamilyId;
use warpui::{AppContext, SingletonEntity};

use super::{AcceptSlashCommandOrSavedPrompt, InlineItem};
use crate::ai::local_brain::custom_agents::LocalCustomAgentStore;
use crate::search::async_snapshot_data_source::AsyncSnapshotDataSource;
use crate::search::data_source::{Query, QueryResult};
use crate::search::mixer::{BoxFuture, DataSourceRunErrorWrapper};
use crate::search::slash_command_menu::fuzzy_match::SlashCommandFuzzyMatchResult;
use crate::search::SearchItem;
use crate::server::server_api::ai::AgentResponse;
use crate::terminal::input::slash_command_model::DetectedCustomAgentCommand;

pub(crate) struct CustomAgentSnapshot {
    query_text: String,
    font_family: FamilyId,
}

pub(crate) fn custom_agents_data_source(
) -> AsyncSnapshotDataSource<CustomAgentSnapshot, AcceptSlashCommandOrSavedPrompt> {
    AsyncSnapshotDataSource::new(
        |query: &Query, app: &AppContext| {
            let appearance = Appearance::as_ref(app);
            CustomAgentSnapshot {
                query_text: query.text.clone(),
                font_family: appearance.monospace_font_family(),
            }
        },
        |snapshot: CustomAgentSnapshot| -> BoxFuture<
            'static,
            Result<Vec<QueryResult<AcceptSlashCommandOrSavedPrompt>>, DataSourceRunErrorWrapper>,
        > {
            Box::pin(async move {
                let agents = LocalCustomAgentStore::list_agents()
                    .await
                    .unwrap_or_default();

                if snapshot.query_text.is_empty() {
                    let mut results = Vec::new();
                    for agent in agents {
                        let item = InlineItem::from_custom_agent(&agent, snapshot.font_family);
                        results.push(QueryResult::from(item));
                    }
                    return Ok(results);
                }

                let mut results = Vec::new();
                for agent in agents {
                    let name = format!("/{}", agent.name);
                    if name
                        .to_lowercase()
                        .contains(&snapshot.query_text.to_lowercase())
                    {
                        let item = InlineItem::from_custom_agent(&agent, snapshot.font_family);
                        results.push(QueryResult::from(item));
                    }
                    yield_now().await;
                }

                Ok(results)
            })
        },
    )
}
