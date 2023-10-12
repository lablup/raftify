from gluesql import Glue, JsonStorage
import json
db = Glue(JsonStorage("json_test"))


def json_pretty_print(path: str):
    with open(path, 'r', encoding='utf-8') as f:
        data = json.load(f)

    with open(path, 'w', encoding='utf-8') as f:
        json.dump(data, f, indent=4, ensure_ascii=False)


entry = {"context": '""', "data": '""', "entry_type": '"EntryNormal"', "index": 1, "term": 1}
# Not working
entry['data'] = """
"ConfChangeV2 { transition: 2, changes: [ConfChangeSingle { change_type: AddNode, node_id: 2 }, ConfChangeSingle { change_type: AddNode, node_id: 3 }], context: \\"['127.0.0.1:60062', '127.0.0.1:60063']\\" }"
""".strip()

print(entry['data'])

# CREATE TABLE Entries;

res = db.query("""
INSERT INTO Entries VALUES ('{{
    "1": {{ "context": {context}, "data": {data}, "entry_type": {entry_type}, "index": {index}, "term": {term} }}
}}');
""".format(**entry))

json_pretty_print("json_test/Entries.jsonl")
