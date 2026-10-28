use std::collections::{HashMap, HashSet};

use super::event::{RichAgentEvent, RichEventId, RichEventStreamId};
use super::projection::RichResponseProjection;

#[derive(Debug)]
#[allow(dead_code)]
pub struct EventDedupSet {
    seen_ids: HashSet<RichEventId>,
}

#[allow(dead_code)]
impl EventDedupSet {
    pub fn new() -> Self {
        Self {
            seen_ids: HashSet::new(),
        }
    }

    pub fn contains(&self, id: &RichEventId) -> bool {
        self.seen_ids.contains(id)
    }

    pub fn insert(&mut self, id: RichEventId) {
        self.seen_ids.insert(id);
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct RichEventReducer {
    pub state: RichResponseProjection,
    pub seen_events: EventDedupSet,
    pub stream_positions: HashMap<RichEventStreamId, u64>,
}

#[allow(dead_code)]
impl RichEventReducer {
    pub fn new() -> Self {
        Self {
            state: RichResponseProjection::default(),
            seen_events: EventDedupSet::new(),
            stream_positions: HashMap::new(),
        }
    }

    pub fn apply(&mut self, event: RichAgentEvent) {
        if self.seen_events.contains(&event.event_id) {
            return;
        }

        let current_sequence = *self.stream_positions.get(&event.stream_id).unwrap_or(&0);
        if event.sequence <= current_sequence && current_sequence > 0 {
            // Drop out-of-order or duplicate sequences unless handling specific out-of-order reconciliation
            // For now, assume sequential stream delivery or buffer handling elsewhere
            return;
        }

        self.stream_positions
            .insert(event.stream_id.clone(), event.sequence);
        self.seen_events.insert(event.event_id.clone());

        // Apply payload to projection
        use super::event::RichEventPayload::*;
        match event.payload {
            Run(r) => {
                self.state.run = Some(super::projection::RunProjection {
                    state: Some(r.state),
                });
            }
            Content(c) => {
                use super::event::ContentEvent;
                match c {
                    ContentEvent::Started { content_id, kind } => {
                        self.state.content.insert(
                            content_id.clone(),
                            super::projection::ContentProjection {
                                content_id,
                                kind: Some(kind),
                                text: String::new(),
                                completed: false,
                            },
                        );
                    }
                    ContentEvent::Delta { content_id, text } => {
                        if let Some(proj) = self.state.content.get_mut(&content_id) {
                            proj.text.push_str(&text);
                        }
                    }
                    ContentEvent::Replaced {
                        content_id,
                        complete_text,
                        ..
                    } => {
                        if let Some(proj) = self.state.content.get_mut(&content_id) {
                            proj.text = complete_text;
                        }
                    }
                    ContentEvent::Completed { content_id } => {
                        if let Some(proj) = self.state.content.get_mut(&content_id) {
                            proj.completed = true;
                        }
                    }
                    _ => {}
                }
            }
            Tool(_) => {
                // To be fleshed out later
            }
            Plan(p) => {
                use super::event::PlanEvent;
                match p {
                    PlanEvent::PlanCreated { plan_id, items } => {
                        self.state.plans.insert(
                            plan_id.clone(),
                            super::projection::PlanProjection {
                                plan_id,
                                items,
                                completed: false,
                            },
                        );
                    }
                    PlanEvent::ItemAdded { plan_id, item } => {
                        if let Some(plan) = self.state.plans.get_mut(&plan_id) {
                            plan.items.push(item);
                        }
                    }
                    PlanEvent::ItemUpdated { plan_id, item } => {
                        if let Some(plan) = self.state.plans.get_mut(&plan_id) {
                            if let Some(existing) = plan.items.iter_mut().find(|i| i.id == item.id)
                            {
                                *existing = item;
                            }
                        }
                    }
                    PlanEvent::ItemCompleted { plan_id, item_id } => {
                        if let Some(plan) = self.state.plans.get_mut(&plan_id) {
                            if let Some(existing) = plan.items.iter_mut().find(|i| i.id == item_id)
                            {
                                existing.completed = true;
                            }
                        }
                    }
                    PlanEvent::PlanCompleted { plan_id } => {
                        if let Some(plan) = self.state.plans.get_mut(&plan_id) {
                            plan.completed = true;
                        }
                    }
                }
            }
            Retrieval(r) => {
                use super::event::RetrievalEvent;
                match r {
                    RetrievalEvent::Started {
                        retrieval_id,
                        safe_query_summary,
                        ..
                    } => {
                        self.state.retrievals.insert(
                            retrieval_id.clone(),
                            super::projection::RetrievalProjection {
                                retrieval_id,
                                status: "started".to_string(),
                                query: safe_query_summary,
                            },
                        );
                    }
                    RetrievalEvent::Completed { retrieval_id } => {
                        if let Some(ret) = self.state.retrievals.get_mut(&retrieval_id) {
                            ret.status = "completed".to_string();
                        }
                    }
                    _ => {}
                }
            }
            Source(s) => {
                self.state.sources.insert(
                    s.source_id.clone(),
                    super::projection::SourceProjection {
                        source_id: s.source_id,
                        name: s.name,
                        path: s.path,
                    },
                );
            }
            Citation(c) => {
                self.state.citations.insert(
                    c.citation_id.clone(),
                    super::projection::CitationProjection {
                        citation_id: c.citation_id,
                        document_type: c.document_type,
                        document_id: c.document_id,
                    },
                );
            }
            Memory(m) => {
                use super::event::MemoryEvent;
                if let MemoryEvent::MemoryUsed {
                    source_id,
                    memory_store_id,
                    safe_label,
                } = m
                {
                    self.state.sources.insert(
                        source_id.clone(),
                        super::projection::SourceProjection {
                            source_id,
                            name: safe_label,
                            path: memory_store_id,
                        },
                    );
                }
            }
            Artifact(art) => {
                self.state.artifacts.insert(
                    art.artifact_id.clone(),
                    super::projection::ArtifactProjection {
                        artifact_id: art.artifact_id,
                        name: art.name,
                        path: art.path,
                        url: art.url,
                    },
                );
            }
            Diff(d) => {
                self.state.diffs.insert(
                    d.path.clone(),
                    super::projection::DiffProjection {
                        path: d.path,
                        diff: d.diff,
                    },
                );
            }
            ChildAgent(ca) => {
                self.state.child_agents.insert(
                    ca.agent_id.clone(),
                    super::projection::ChildAgentProjection {
                        agent_id: ca.agent_id,
                        name: ca.name,
                        status: ca.status,
                    },
                );
            }
            Routing(r) => {
                use super::event::RoutingEvent;
                if let RoutingEvent::RouteSelected { provider, model } = r {
                    self.state.routing =
                        Some(super::projection::RoutingProjection { provider, model });
                }
            }
            Context(_) => {
                self.state.context = Some(super::projection::ContextProjection::default());
            }
            Usage(u) => {
                self.state.usage = Some(super::projection::UsageProjection {
                    prompt_tokens: u.prompt_tokens,
                    completion_tokens: u.completion_tokens,
                    total_tokens: u.total_tokens,
                });
            }
            Suggestion(s) => {
                self.state.suggestions.push(s.suggestion);
            }
            Warning(_) => {}
            Terminal(t) => {
                self.state.terminal = Some(t.outcome);
            }
            Status(_) => {}
            ReasoningSummary(_) => {}
        }
    }
}
