import pickle
from typing import Optional

from rraft import (
    set_confchange_context_deserializer,
    set_confchangev2_context_deserializer,
    set_entry_context_deserializer,
    set_entry_data_deserializer,
    set_message_context_deserializer,
    set_snapshot_data_deserializer,
)


def pickle_deserialize(data: bytes) -> Optional[str]:
    if data == b"":
        return None

    if pickle.PROTO in data:
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
    set_entry_data_deserializer(pickle_deserialize)
    set_message_context_deserializer(pickle_deserialize)
    set_snapshot_data_deserializer(pickle_deserialize)
