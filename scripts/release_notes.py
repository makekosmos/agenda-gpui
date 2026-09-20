"""Release notes from shipped commits/PRs, optionally enriched with Linear titles."""

import argparse
import json
import os
from pathlib import Path
import re
import urllib.request

from release import run


ISSUE = re.compile(r"\bKOS-[0-9]+\b", re.IGNORECASE)
LINEAR_URL = "https://linear.app/yosokosmos/issue/"


def github(path, paginate=False):
    args = ["gh", "api", f"repos/{os.environ['GITHUB_REPOSITORY']}/{path}"]
    if paginate:
        pages = json.loads(run(*args, "--paginate", "--slurp"))
        return [item for page in pages for item in page]
    return json.loads(run(*args))


def shipped_changes(previous, head):
    revision = f"{previous}..{head}" if previous else head
    shas = run("git", "rev-list", "--reverse", revision).splitlines()
    sha_set = set(shas)
    seen_prs = set()
    changes = []
    for sha in shas:
        prs = github(f"commits/{sha}/pulls?per_page=100", paginate=True)
        prs = [pr for pr in prs if pr.get("merged_at") and pr.get("merge_commit_sha") in sha_set
               and pr["base"]["repo"]["full_name"] == os.environ["GITHUB_REPOSITORY"]]
        if prs:
            for pr in prs:
                if pr["number"] in seen_prs:
                    continue
                seen_prs.add(pr["number"])
                pr = github(f"pulls/{pr['number']}")
                # Prefer the task being implemented over incidental links in the PR body.
                ids = ISSUE.findall(pr["title"] + " " + pr["head"]["ref"])
                if not ids:
                    ids = re.findall(r"https://linear\.app/[^/\s]+/issue/(KOS-[0-9]+)\b", pr.get("body") or "", re.I)
                changes.append({
                    "ids": sorted(set(i.upper() for i in ids)), "title": pr["title"],
                    "author": "@" + pr["user"]["login"],
                    "ref": f"#{pr['number']}", "url": pr["html_url"],
                    "merger": "@" + pr["merged_by"]["login"] if pr.get("merged_by") else None,
                })
        else:
            commit = github(f"commits/{sha}")
            title = commit["commit"]["message"].splitlines()[0]
            if re.fullmatch(r"chore: release v[0-9]+\.[0-9]+\.[0-9]+", title):
                continue
            author = commit.get("author")
            changes.append({
                "ids": sorted(set(i.upper() for i in ISSUE.findall(title))), "title": title,
                "author": "@" + author["login"] if author else commit["commit"]["author"]["name"],
                "ref": sha[:7], "url": commit["html_url"], "merger": None,
            })
    return changes


def linear_titles(identifiers):
    key = os.environ.get("LINEAR_API_KEY")
    if not key:
        if identifiers:
            print("::warning::LINEAR_API_KEY is not configured; using PR/commit titles for Linear tasks")
        return {}
    result = {}
    for identifier in sorted(identifiers):
        payload = {
            "query": "query($id: String!) { issue(id: $id) { title } }",
            "variables": {"id": identifier},
        }
        request = urllib.request.Request(
            "https://api.linear.app/graphql", data=json.dumps(payload).encode(),
            headers={"Authorization": key, "Content-Type": "application/json"},
        )
        with urllib.request.urlopen(request, timeout=30) as response:
            data = json.load(response)
        # Don't publish partial/misleading notes if a configured integration fails.
        issue = (data.get("data") or {}).get("issue")
        if data.get("errors") or not issue:
            raise RuntimeError(f"Cannot read Linear issue {identifier}; check LINEAR_API_KEY access")
        result[identifier] = issue["title"]
    return result


def markdown(value):
    value = " ".join(value.split())
    return re.sub(r"([\\`*_<>{}\[\]])", r"\\\1", value).replace("@", "&#64;")


def author_text(author):
    if re.fullmatch(r"@[A-Za-z0-9-]+(?:\[bot\])?", author):
        return author
    return markdown(author)


def reference(change):
    link = f"[{change['ref']}]({change['url']})"
    if change["merger"]:
        link += f" (слил {author_text(change['merger'])})"
    return link


def render(changes, titles, previous, head, repository):
    tasks = {}
    other = []
    for change in changes:
        if not change["ids"]:
            other.append(change)
        for identifier in change["ids"]:
            tasks.setdefault(identifier, []).append(change)
    lines = ["## Задачи в релизе", ""]
    if not tasks:
        lines.append("В вошедших изменениях нет ссылок на задачи Linear.")
    for identifier, items in tasks.items():
        fallback = ISSUE.sub("", items[0]["title"]).strip(" []():-") or "Изменения по задаче"
        title = markdown(titles.get(identifier, fallback))
        authors = ", ".join(dict.fromkeys(author_text(item["author"]) for item in items))
        refs = ", ".join(dict.fromkeys(reference(item) for item in items))
        lines.append(f"- [{identifier}]({LINEAR_URL}{identifier}) {title} {authors} — {refs}")
    if other:
        lines += ["", "## Другие изменения", ""]
        for change in other:
            lines.append(f"- {markdown(change['title'])} {author_text(change['author'])} — {reference(change)}")
    path = f"compare/{previous}...{head}" if previous else f"commits/{head}"
    lines += ["", f"[Все изменения](https://github.com/{repository}/{path})", ""]
    return "\n".join(lines)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--previous", default="")
    parser.add_argument("--head", required=True)
    parser.add_argument("--output", default="release-notes.md")
    args = parser.parse_args()
    changes = shipped_changes(args.previous, args.head)
    identifiers = {identifier for change in changes for identifier in change["ids"]}
    notes = render(changes, linear_titles(identifiers), args.previous, args.head, os.environ["GITHUB_REPOSITORY"])
    Path(args.output).write_text(notes, encoding="utf-8", newline="\n")
