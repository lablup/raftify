from gluesql import Glue, JsonStorage
import json
db = Glue(JsonStorage("json_test"))


def json_pretty_print(path: str):
    with open(path, 'r', encoding='utf-8') as f:
        data = json.load(f)

    with open(path, 'w', encoding='utf-8') as f:
        json.dump(data, f, indent=4, ensure_ascii=False)


hard_state = {
    "commit": 3,
    "term": 3,
    "vote": 3,
}

conf_state = {
    "voters": [1, 2, 3],
    "learners": [4, 5, 6],
    "voters_outgoing": [],
    "learners_next": [],
    "auto_leave": "false",
}

others = {
    "last_index": 3
}

snapshot = {
    "data": b"",
    "metadata": {
        "conf_state": conf_state,
        "others": others,
        "index": 3,
        "term": 3
    }
}

flatten_snapshot = {
    "snapshot_metadata_voters": [1, 2, 3],
    "snapshot_metadata_learners": [4, 5, 6],
    "snapshot_metadata_voters_outgoing": [],
    "snapshot_metadata_learners_next": [],
}


# DB를 처음 만들 때 INSERT 코드로 한 개의 Metadata를 넣어주자
# CREATE TABLE Metadata;

res = db.query("""
INSERT INTO Metadata VALUES ('{{
    "hard_state": {{ "commit": {commit}, "term": {term}, "vote": {vote} }},
    "conf_state": {{ "voters": {voters}, "learners": {learners}, "voters_outgoing": {voters_outgoing}, "learners_next": {learners_next}, "auto_leave": {auto_leave} }},
    "last_index": {last_index},
    "snapshot": {{
        "metadata": {{
            "conf_state": {{
                "voters": {snapshot_metadata_voters},
                "learners": {snapshot_metadata_learners},
                "voters_outgoing": {snapshot_metadata_voters_outgoing},
                "learners_next": {snapshot_metadata_learners_next}
            }}
        }}
    }}
}}');
""".format(**hard_state, **conf_state, **others, **flatten_snapshot))

json_pretty_print("json_test/Metadata.jsonl")

# assert res[0].affected == 1

# magic happens here to make it pretty-printed


# pprintjson.pprintjson(json_file.readlines())

# mydata = json.loads(output)

# # DB를 읽어올 때
# res = db.query("""
# SELECT * FROM Metadata;
# """)
# print(res)

# DB 업데이트

# db.query("""
# INSERT INTO Logs VALUES ('{{ "commit": {commit}, "term": {term}, "vote": {vote} }}');
# """.format(**hard_state))
