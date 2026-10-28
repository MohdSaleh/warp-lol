use super::*;

#[test]
fn test_graph_engine_transitions() {
    let mut engine = GraphEngine::new(NodeId::FileIndexer, 3);
    assert_eq!(engine.state.current_node, NodeId::FileIndexer);

    // Transition to TerminalParser
    let res = engine.transition(Transition::Next(NodeId::TerminalParser));
    assert_eq!(res, Ok(NodeId::TerminalParser));
    assert_eq!(engine.state.step_count, 1);
    assert_eq!(engine.state.history_nodes, vec![NodeId::FileIndexer]);

    // Transition cycle
    let res = engine.transition(Transition::Cycle);
    assert_eq!(res, Ok(NodeId::TerminalParser));
    assert_eq!(engine.state.step_count, 2);

    // Transition to CodeGenerator
    let res = engine.transition(Transition::Next(NodeId::CodeGenerator));
    assert_eq!(res, Ok(NodeId::CodeGenerator));
    assert_eq!(engine.state.step_count, 3);

    // Max steps reached
    let res = engine.transition(Transition::Next(NodeId::Reviewer));
    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err(),
        "Max graph execution steps reached (preventing infinite recursion)"
    );
}

#[test]
fn test_graph_engine_termination() {
    let mut engine = GraphEngine::new(NodeId::FileIndexer, 5);
    let res = engine.transition(Transition::Terminate("Completed task".into()));
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Completed task"));
}
