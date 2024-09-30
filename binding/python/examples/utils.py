import os


def get_storage_path(log_dir: str, node_id: int) -> str:
    return f"{log_dir}/node-{node_id}"


def ensure_directory_exist(storage_path: str):
    if not os.path.exists(storage_path):
        os.makedirs(storage_path)
