#![allow(non_snake_case)]
use ::raftify::raft::formatter::set_custom_formatter;
use bindings::formatter::PythonFormatter;
use pyo3::prelude::*;

mod bindings;

#[pymodule]
fn raftify(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<bindings::config::PyConfig>()?;
    m.add_class::<bindings::raft_rs::config::PyRaftConfig>()?;
    m.add_class::<bindings::raft_facade::PyRaftFacade>()?;
    m.add_class::<bindings::peers::PyPeers>()?;
    m.add_class::<bindings::raft_client::PyRaftServiceClient>()?;
    m.add_class::<bindings::raft_node::PyRaftNode>()?;
    m.add_class::<bindings::logger::PyLogger>()?;
    m.add_class::<bindings::logger::PyOverflowStrategy>()?;
    m.add_class::<bindings::cluster_join_ticket::PyClusterJoinTicket>()?;
    m.add_class::<bindings::raft_rs::readonly_option::PyReadOnlyOption>()?;

    m.add_class::<bindings::raft_rs::eraftpb::conf_change_single::PyConfChangeSingle>()?;
    m.add_class::<bindings::raft_rs::eraftpb::conf_change_transition::PyConfChangeTransition>()?;
    m.add_class::<bindings::raft_rs::eraftpb::conf_change_type::PyConfChangeType>()?;
    m.add_class::<bindings::raft_rs::eraftpb::conf_change_v2::PyConfChangeV2>()?;
    m.add_class::<bindings::raft_rs::eraftpb::message::PyMessage>()?;
    m.add_class::<bindings::raft_rs::eraftpb::message_type::PyMessageType>()?;
    m.add_class::<bindings::raft_rs::eraftpb::entry::PyEntry>()?;
    m.add_class::<bindings::raft_rs::eraftpb::entry_type::PyEntryType>()?;

    m.add_function(wrap_pyfunction!(bindings::cli::cli_main, m)?)?;

    m.add_function(wrap_pyfunction!(
        bindings::state_machine::set_log_entry_deserializer,
        m
    )?)?;

    m.add_function(wrap_pyfunction!(
        bindings::state_machine::set_fsm_deserializer,
        m
    )?)?;

    m.add_function(wrap_pyfunction!(
        bindings::formatter::set_confchange_context_deserializer,
        m
    )?)?;

    m.add_function(wrap_pyfunction!(
        bindings::formatter::set_confchangev2_context_deserializer,
        m
    )?)?;

    m.add_function(wrap_pyfunction!(
        bindings::formatter::set_entry_context_deserializer,
        m
    )?)?;

    m.add_function(wrap_pyfunction!(
        bindings::formatter::set_entry_data_deserializer,
        m
    )?)?;

    m.add_function(wrap_pyfunction!(
        bindings::formatter::set_message_context_deserializer,
        m
    )?)?;

    m.add_function(wrap_pyfunction!(
        bindings::formatter::set_snapshot_data_deserializer,
        m
    )?)?;

    m.add("ApplyError", py.get_type::<bindings::errors::ApplyError>())?;
    m.add(
        "DecodingError",
        py.get_type::<bindings::errors::DecodingError>(),
    )?;
    m.add(
        "EncodingError",
        py.get_type::<bindings::errors::EncodingError>(),
    )?;
    m.add(
        "RestoreError",
        py.get_type::<bindings::errors::RestoreError>(),
    )?;
    m.add(
        "WrongArgumentError",
        py.get_type::<bindings::errors::WrongArgumentError>(),
    )?;
    m.add(
        "SnapshotError",
        py.get_type::<bindings::errors::SnapshotError>(),
    )?;

    set_custom_formatter(PythonFormatter);

    Ok(())
}
