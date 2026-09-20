"""Run with python scripts/test_release_notes.py; no network or credentials needed."""

import io
import json
import os
from unittest.mock import patch

import release_notes as notes


repo = "example/agenda"
pr = {
    "number": 7, "merged_at": "2026-09-20", "merge_commit_sha": "merge",
    "base": {"repo": {"full_name": repo}}, "title": "Fix timer",
    "head": {"ref": "kos-123-timer"}, "body": "Related: KOS-999",
    "user": {"login": "alice"}, "merged_by": {"login": "bob"},
    "html_url": "https://github.com/example/agenda/pull/7",
}


def commit(title, login="carol"):
    return {
        "commit": {"message": title, "author": {"name": "Local Author"}},
        "author": {"login": login} if login else None,
        "html_url": "https://github.com/example/agenda/commit/direct",
    }


responses = {
    "commits/feature/pulls?per_page=100": [pr],
    "commits/merge/pulls?per_page=100": [pr],
    "pulls/7": pr,
    "commits/direct/pulls?per_page=100": [
        {**pr, "merged_at": None}, {**pr, "merge_commit_sha": "old-history"},
        {**pr, "base": {"repo": {"full_name": "other/repo"}}},
    ],
    "commits/direct": commit("KOS-123: Timer follow-up"),
    "commits/local/pulls?per_page=100": [],
    "commits/local": commit("Docs\nPrivate commit body", None),
    "commits/version/pulls?per_page=100": [],
    "commits/version": commit("chore: release v0.1.1"),
}
with patch.dict(os.environ, {"GITHUB_REPOSITORY": repo}), \
        patch.object(notes, "run", return_value="feature\nmerge\ndirect\nlocal\nversion") as run, \
        patch.object(notes, "github", side_effect=lambda path, **kw: responses[path]) as github:
    changes = notes.shipped_changes("v0.1.0", "head")
    run.assert_called_once_with("git", "rev-list", "--reverse", "v0.1.0..head")
    assert len(changes) == 3
    assert changes[0]["ids"] == ["KOS-123"]
    assert changes[0]["author"] == "@alice" and changes[0]["merger"] == "@bob"
    assert sum(call.args == ("pulls/7",) for call in github.call_args_list) == 1
    assert changes[2]["author"] == "Local Author"

text = notes.render(changes, {"KOS-123": "Точный заголовок"}, "v0.1.0", "head", repo)
assert text.count("- [KOS-123]") == 1
assert "Точный заголовок @alice, @carol" in text
assert "слил @bob" in text and "[#7]" in text
assert "KOS-999" not in text and "chore: release" not in text
assert "Docs Local Author" in text and "@Local" not in text and "Private" not in text
assert "compare/v0.1.0...head" in text
fallback = notes.render(changes, {}, "", "head", repo)
assert "Fix timer @alice" in fallback and "commits/head" in fallback
assert "В вошедших изменениях нет" in notes.render([], {}, "", "head", repo)
assert notes.markdown("[evil](url)\n@everyone <script>") == r"\[evil\](url) &#64;everyone \<script\>"

# Only a Linear URL in the body is a fallback; unrelated plain IDs aren't tasks.
with patch.dict(os.environ, {"GITHUB_REPOSITORY": repo}), \
        patch.object(notes, "run", return_value="merge"), \
        patch.object(notes, "github", side_effect=[
            [pr], {**pr, "head": {"ref": "fix"}, "body": "KOS-999 https://linear.app/yosokosmos/issue/KOS-42/title"},
        ]):
    assert notes.shipped_changes("", "head")[0]["ids"] == ["KOS-42"]

with patch.dict(os.environ, {}, clear=True), patch.object(notes.urllib.request, "urlopen") as fetch:
    assert notes.linear_titles({"KOS-123"}) == {}
    fetch.assert_not_called()

with patch.dict(os.environ, {"LINEAR_API_KEY": "test-key"}), \
        patch.object(notes.urllib.request, "urlopen", return_value=io.BytesIO(
            json.dumps({"data": {"issue": {"title": "Linear title"}}}).encode()
        )) as fetch:
    assert notes.linear_titles({"KOS-123"}) == {"KOS-123": "Linear title"}
    request = fetch.call_args.args[0]
    assert request.get_header("Authorization") == "test-key"
    assert json.loads(request.data)["variables"] == {"id": "KOS-123"}

with patch.dict(os.environ, {"LINEAR_API_KEY": "test-key"}), \
        patch.object(notes.urllib.request, "urlopen", return_value=io.BytesIO(b'{"errors":[{"message":"denied"}]}')):
    try:
        notes.linear_titles({"KOS-123"})
        raise AssertionError("GraphQL errors must stop publication")
    except RuntimeError as error:
        assert "KOS-123" in str(error) and "test-key" not in str(error)

print("Release notes checks passed")
