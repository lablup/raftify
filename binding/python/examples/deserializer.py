import pickle
from raftify import (
    set_confchange_context_deserializer,
    set_confchangev2_context_deserializer,
    set_entry_context_deserializer,
    set_entry_data_deserializer,
    set_message_context_deserializer,
    set_snapshot_data_deserializer,
)


def pickle_deserialize(data: bytes) -> str | None:
    if data == b"":
        return None

    if pickle.PROTO in data:
        r = pickle.loads(data[data.index(pickle.PROTO) :])
        return r

    # Not pickle data
    return None


def register_custom_deserializer() -> None:
    """
    Initialize the custom deserializers.
    """

    set_confchange_context_deserializer(pickle_deserialize)
    set_confchangev2_context_deserializer(pickle_deserialize)
    set_entry_context_deserializer(pickle_deserialize)
    set_entry_data_deserializer(pickle_deserialize)
    set_message_context_deserializer(pickle_deserialize)
    set_snapshot_data_deserializer(pickle_deserialize)
