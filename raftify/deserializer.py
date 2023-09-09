import pickle
from typing import Optional

from rraft import (
    ConfChange,
    set_confchange_context_deserializer,
    set_confchangev2_context_deserializer,
    set_entry_context_deserializer,
    set_entry_data_deserializer,
    set_message_context_deserializer,
    set_snapshot_data_deserializer,
)


def pickle_deserialize(data: bytes) -> Optional[str]:
    """
    This function assumes that the given byte slice has been dumped with pickle and perform deserialization.
    Note that if the given byte slice is not data dumped with pickle, but it contains data that has been dumped with pickle,
    This returns only the dumped portion with pickle.
    """

    if data == b"":
        return None

    if pickle.PROTO in data:
        return pickle.loads(data[data.index(pickle.PROTO) :])

    # Not pickle data
    return data


def set_entry_data_deserializer_cb(data: bytes) -> Optional[str]:
    if data == b"":
        return None

    try:
        return str(ConfChange.decode(data))
    except Exception:
        pass

    if data.startswith(pickle.PROTO):
        return pickle.loads(data[data.index(pickle.PROTO) :])

    # Not pickle data
    return data


def init_rraft_py_deserializer():
    """
    Initialize the deserializers using in rraft-py.
    """

    set_confchange_context_deserializer(pickle_deserialize)
    set_confchangev2_context_deserializer(pickle_deserialize)
    set_entry_context_deserializer(pickle_deserialize)
    set_entry_data_deserializer(set_entry_data_deserializer_cb)
    set_message_context_deserializer(pickle_deserialize)
    set_snapshot_data_deserializer(pickle_deserialize)
