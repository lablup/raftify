import pickle
from raftify import set_custom_formatters


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

    set_custom_formatters(
        entry_data=pickle_deserialize,
        entry_context=pickle_deserialize,
        confchange_context=pickle_deserialize,
        confchangev2_context=pickle_deserialize,
        message_context=pickle_deserialize,
        snapshot_data=pickle_deserialize,
        log_entry=pickle_deserialize,
        fsm=pickle_deserialize,
    )
