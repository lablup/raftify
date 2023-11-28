import pickle

from rraft import (
    ConfChangeV2,
    set_confchange_context_deserializer,
    set_confchangev2_context_deserializer,
    set_entry_context_deserializer,
    set_entry_data_deserializer,
    set_message_context_deserializer,
    set_snapshot_data_deserializer,
)


def pickle_deserialize(data: bytes) -> str | bytes:
    """
    This function assumes that the given byte slice has been dumped with pickle and perform deserialization.
    Note that if the given byte slice is not data dumped with pickle, but it contains data that has been dumped with pickle,
    This returns only the dumped portion with pickle.
    """

    if data == b"":
        return "None"

    if pickle.PROTO in data:
        return pickle.loads(data[data.index(pickle.PROTO) :])

    # Not pickle data
    return data


def entry_data_deserializer(data: bytes) -> str | bytes:
    if data == b"":
        return "None"

    try:
        return str(ConfChangeV2.decode(data))
    except Exception:
        pass

    if data.startswith(pickle.PROTO):
        return pickle.loads(data[data.index(pickle.PROTO) :])

    # Not pickle data
    return data


def init_rraft_py_deserializer() -> None:
    """
    Initialize the custom deserializers using in rraft-py.

    This function Assume that all deserializers use pickle for serialization and deserialization.
    It's not mandatory for the caller to invoke this function, but if not invoked, byte slices will be logged without being deserialized.
    So, if you don't want to call this function, it would be good to manually set the deserializers in rraft_py.
    """

    set_confchange_context_deserializer(pickle_deserialize)
    set_confchangev2_context_deserializer(pickle_deserialize)
    set_entry_context_deserializer(pickle_deserialize)
    set_entry_data_deserializer(entry_data_deserializer)
    set_message_context_deserializer(pickle_deserialize)
    set_snapshot_data_deserializer(pickle_deserialize)
