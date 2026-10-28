use anyhow::Result;
use diesel::prelude::*;
use diesel::SqliteConnection;
use persistence::model::{
    OrchestrationRouterEdgeRecord, OrchestrationRouterOutboxRecord, OrchestrationRouterRunRecord,
};
use persistence::schema::{
    orchestration_router_edges, orchestration_router_outbox, orchestration_router_runs,
};

pub fn persist_run(
    conn: &mut SqliteConnection,
    record: OrchestrationRouterRunRecord,
) -> Result<()> {
    diesel::insert_into(orchestration_router_runs::table)
        .values(&record)
        .on_conflict(orchestration_router_runs::run_id)
        .do_update()
        .set((
            orchestration_router_runs::lifecycle_state.eq(&record.lifecycle_state),
            orchestration_router_runs::last_source_sequence.eq(&record.last_source_sequence),
            orchestration_router_runs::last_ingest_sequence.eq(&record.last_ingest_sequence),
            orchestration_router_runs::updated_at.eq(&record.updated_at),
            orchestration_router_runs::terminal_reason.eq(&record.terminal_reason),
        ))
        .execute(conn)?;
    Ok(())
}

pub fn persist_edge(
    conn: &mut SqliteConnection,
    record: OrchestrationRouterEdgeRecord,
) -> Result<()> {
    diesel::insert_into(orchestration_router_edges::table)
        .values(&record)
        .on_conflict_do_nothing()
        .execute(conn)?;
    Ok(())
}

pub fn append_outbox(
    conn: &mut SqliteConnection,
    record: OrchestrationRouterOutboxRecord,
) -> Result<()> {
    diesel::insert_into(orchestration_router_outbox::table)
        .values(&record)
        .on_conflict_do_nothing()
        .execute(conn)?;
    Ok(())
}

pub fn delete_outbox(conn: &mut SqliteConnection, event_id: &str) -> Result<()> {
    diesel::delete(
        orchestration_router_outbox::table
            .filter(orchestration_router_outbox::event_id.eq(event_id)),
    )
    .execute(conn)?;
    Ok(())
}

pub fn load_all_runs(conn: &mut SqliteConnection) -> Result<Vec<OrchestrationRouterRunRecord>> {
    let results = orchestration_router_runs::table.load::<OrchestrationRouterRunRecord>(conn)?;
    Ok(results)
}

pub fn load_all_edges(conn: &mut SqliteConnection) -> Result<Vec<OrchestrationRouterEdgeRecord>> {
    let results = orchestration_router_edges::table.load::<OrchestrationRouterEdgeRecord>(conn)?;
    Ok(results)
}
