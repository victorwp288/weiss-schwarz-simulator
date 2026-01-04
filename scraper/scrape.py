#!/usr/bin/env python3
import asyncio
import json
import os
import re
import sys
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Sequence, Tuple
from urllib.parse import parse_qs, urljoin, urlparse

import httpx
from bs4 import BeautifulSoup, NavigableString, Tag

BASE_URL = "https://en.ws-tcg.com"
CARDLIST_URL = f"{BASE_URL}/cardlist/"
LISTING_ENDPOINT = f"{BASE_URL}/cardlist/cardsearch_ex"
DETAIL_URL_TEMPLATE = f"{BASE_URL}/cardlist/?cardno={{cardno}}"

SCHEMA_VERSION = "1"
USER_AGENT = "Mozilla/5.0 (compatible; WeissSchwarzScraper/1.0; +https://en.ws-tcg.com/cardlist/)"

BASE_LISTING_PARAMS = {
    "keyword": "",
    "keyword_or": "",
    "keyword_not": "",
    "keyword_type[0]": "name",
    "keyword_type[1]": "feature",
    "keyword_type[2]": "text",
    "keyword_type[3]": "no",
    "side": "",
    "title": "",
    "category": "",
    "expansion_name": "",
    "card_kind[0]": "all",
    "color[0]": "all",
    "level_s": "",
    "level_e": "",
    "power_s": "",
    "power_e": "",
    "soul_s": "",
    "soul_e": "",
    "cost_s": "",
    "cost_e": "",
    "trigger": "",
    "view": "image",
    "page": "1",
}

DASH_VALUES = {"", "-", "－", "—", "–"}
VARIANT_RE = re.compile(r"(?<=\d)[A-Z]+$")


def now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def ensure_out_dir(out_dir: Path) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)


def abs_url(path_or_url: Optional[str]) -> str:
    if not path_or_url:
        return ""
    if path_or_url.startswith("http://") or path_or_url.startswith("https://"):
        return path_or_url
    return urljoin(BASE_URL, path_or_url)


def normalize_dash(text: Optional[str]) -> Optional[str]:
    if text is None:
        return None
    stripped = text.strip()
    if stripped in DASH_VALUES:
        return None
    return stripped


def parse_int_field(text: Optional[str]) -> Optional[int]:
    text = normalize_dash(text)
    if text is None:
        return None
    try:
        return int(text)
    except ValueError:
        return None


def split_traits(raw: Optional[str]) -> List[str]:
    raw = normalize_dash(raw)
    if not raw:
        return []
    parts = re.split(r"[・/／]", raw)
    return [p.strip() for p in parts if p.strip()]


def base_cardno(cardno: str) -> str:
    return VARIANT_RE.sub("", cardno)


def is_variant(cardno: str) -> bool:
    return VARIANT_RE.search(cardno) is not None


def filename_from_src(src: Optional[str]) -> Optional[str]:
    if not src:
        return None
    return os.path.basename(urlparse(src).path)


ICON_TOKEN_MAP = {
    "choice": "CHOICE",
    "gate": "GATE",
    "bounce": "BOUNCE",
    "treasure": "TREASURE",
    "shot": "SHOT",
    "standby": "STANDBY",
    "soul": "SOUL",
    "soul2": "SOUL2",
    "stock": "POOL",
    "pool": "POOL",
}


def normalize_side(value: Optional[str]) -> str:
    if not value:
        return ""
    value = value.strip()
    if value.lower().startswith("w"):
        return "W"
    if value.lower().startswith("s"):
        return "S"
    if "wei" in value.lower():
        return "W"
    if "schwarz" in value.lower():
        return "S"
    return ""


def normalize_color(value: Optional[str]) -> str:
    if not value:
        return ""
    token = value.strip()
    lowered = token.lower()
    mapping = {
        "yellow": "Yellow",
        "green": "Green",
        "red": "Red",
        "blue": "Blue",
    }
    return mapping.get(lowered, token.title())


def normalize_card_type(value: Optional[str]) -> str:
    if not value:
        return ""
    token = value.strip()
    if not token:
        return ""
    lowered = token.lower()
    mapping = {
        "character": "Character",
        "event": "Event",
        "climax": "Climax",
    }
    return mapping.get(lowered, token)


def normalize_trigger(value: str) -> str:
    token = value.strip()
    if not token:
        return ""
    value = token.lower()
    mapping = {
        "soul": "Soul",
        "soul2": "Soul2",
        "choice": "Choice",
        "gate": "Gate",
        "door": "Gate",
        "bounce": "Bounce",
        "treasure": "Treasure",
        "shot": "Shot",
        "standby": "Standby",
        "draw": "Draw",
        "book": "Draw",
        "pool": "Pool",
        "stock": "Pool",
    }
    return mapping.get(value, token.title())


def normalize_name(value: str) -> str:
    if not value:
        return value
    # Known typo fixups
    value = value.replace("Yotusba Nakano", "Yotsuba Nakano")
    return value


def token_for_img(img: Tag) -> str:
    src = img.get("src") if img else None
    stem = ""
    if src:
        fname = filename_from_src(src)
        if fname:
            stem = os.path.splitext(fname)[0].lower()
    token = ICON_TOKEN_MAP.get(stem, None)
    if not token:
        if stem:
            token = f"ICON:{stem.upper()}"
        else:
            token = "ICON"
    return f" [{token}] "


def listing_params(page: int, **overrides: str) -> Dict[str, str]:
    params = dict(BASE_LISTING_PARAMS)
    params.update({k: v for k, v in overrides.items() if v is not None})
    params["page"] = str(page)
    return params


def parse_listing_cardnos(html: str) -> List[str]:
    soup = BeautifulSoup(html, "html.parser")
    cardnos: List[str] = []
    seen = set()
    for a in soup.find_all("a", href=True):
        href = a["href"]
        if "cardno=" not in href:
            continue
        parsed = urlparse(href)
        qs = parse_qs(parsed.query)
        cardno_list = qs.get("cardno")
        if not cardno_list:
            continue
        cardno = cardno_list[0].strip()
        if not cardno:
            continue
        cardno = base_cardno(cardno)
        if cardno and cardno not in seen:
            seen.add(cardno)
            cardnos.append(cardno)
    return cardnos


def extract_select_options(html: str) -> Dict[str, List[Tuple[str, str]]]:
    soup = BeautifulSoup(html, "html.parser")
    selects = {}
    for name in ("title", "expansion_name", "category"):
        sel = soup.find("select", {"name": name})
        options: List[Tuple[str, str]] = []
        if sel:
            for opt in sel.find_all("option"):
                value = (opt.get("value") or "").strip()
                label = (opt.text or "").strip()
                if value:
                    options.append((value, label))
        selects[name] = options
    return selects


def text_from_tag(tag) -> Optional[str]:
    if tag is None:
        return None
    return tag.get_text(strip=True)


def text_with_tokens(tag: Optional[Tag]) -> str:
    if tag is None:
        return ""
    parts: List[str] = []

    def walk(node) -> None:
        if isinstance(node, NavigableString):
            parts.append(str(node))
            return
        if isinstance(node, Tag):
            if node.name == "br":
                parts.append("\n")
                return
            if node.name == "img":
                parts.append(token_for_img(node))
                return
            for child in node.children:
                walk(child)

    walk(tag)
    text = "".join(parts).replace("\r\n", "\n").replace("\r", "\n")
    lines = []
    for line in text.split("\n"):
        line = re.sub(r"\s+", " ", line).strip()
        if line:
            lines.append(line)
    return "\n".join(lines)


def normalize_block_text(text: str) -> str:
    text = text.replace("\r\n", "\n").replace("\r", "\n").strip()
    text = re.sub(r"[ \t]+", " ", text)
    text = re.sub(r"\n{2,}", "\n", text)
    return text


def parse_detail_html(html: str, requested_cardno: str) -> Tuple[Optional[dict], Optional[str]]:
    soup = BeautifulSoup(html, "html.parser")
    wrapper = soup.find("div", class_="p-cards__detail-wrapper-inner")
    if not wrapper:
        return None, "detail_wrapper_missing"

    number = text_from_tag(wrapper.find("p", class_="number"))
    name = normalize_name(text_from_tag(wrapper.find("p", class_="ttl")) or "")
    image_src = None
    image_div = wrapper.find("div", class_="image")
    if image_div:
        img = image_div.find("img")
        if img:
            image_src = img.get("src")

    if number and base_cardno(number) != base_cardno(requested_cardno):
        return None, f"cardno_mismatch:{number}"

    type_pairs: Dict[str, Any] = {}
    status_pairs: Dict[str, Any] = {}

    type_container = wrapper.find("div", class_="p-cards__detail-type")
    if type_container:
        for dl in type_container.find_all("dl"):
            dt = dl.find("dt")
            dd = dl.find("dd")
            if dt and dd:
                type_pairs[dt.get_text(strip=True)] = dd

    status_container = wrapper.find("div", class_="p-cards__detail-status")
    if status_container:
        for dl in status_container.find_all("dl"):
            dt = dl.find("dt")
            dd = dl.find("dd")
            if dt and dd:
                status_pairs[dt.get_text(strip=True)] = dd

    expansion_raw = text_from_tag(type_pairs.get("Expansion")) or ""
    traits_raw = text_from_tag(type_pairs.get("Traits")) or ""
    card_type = normalize_card_type(text_from_tag(type_pairs.get("Card Type")))
    rarity = text_from_tag(type_pairs.get("Rarity")) or ""

    side = ""
    side_dd = type_pairs.get("Side")
    if side_dd:
        img = side_dd.find("img") if hasattr(side_dd, "find") else None
        side_token = (
            filename_from_src(img.get("src") if img else None) or text_from_tag(side_dd) or ""
        )
        if side_token:
            side = normalize_side(side_token.split(".")[0])

    color = ""
    color_dd = type_pairs.get("Color")
    if color_dd:
        img = color_dd.find("img") if hasattr(color_dd, "find") else None
        color_token = (
            filename_from_src(img.get("src") if img else None) or text_from_tag(color_dd) or ""
        )
        if color_token:
            color = normalize_color(os.path.splitext(color_token)[0])

    level = parse_int_field(text_from_tag(status_pairs.get("Level")))
    cost = parse_int_field(text_from_tag(status_pairs.get("Cost")))
    power = parse_int_field(text_from_tag(status_pairs.get("Power")))

    trigger_dd = status_pairs.get("Trigger")
    triggers: List[str] = []
    if trigger_dd:
        imgs = trigger_dd.find_all("img") if hasattr(trigger_dd, "find_all") else []
        if imgs:
            for img in imgs:
                fname = filename_from_src(img.get("src"))
                if fname:
                    stem = os.path.splitext(fname)[0]
                    triggers.append(normalize_trigger(stem))
        else:
            text = normalize_dash(text_from_tag(trigger_dd))
            if text:
                triggers.append(normalize_trigger(text))

    soul_dd = status_pairs.get("Soul")
    soul: Optional[int] = None
    if soul_dd:
        imgs = soul_dd.find_all("img") if hasattr(soul_dd, "find_all") else []
        if imgs:
            soul = len(imgs)
        else:
            soul = parse_int_field(text_from_tag(soul_dd))

    detail_blocks = wrapper.select("div.p-cards__detail")
    text_lines: List[str] = []
    seen_blocks: set = set()
    for block in detail_blocks:
        chunk = text_with_tokens(block)
        if not chunk:
            continue
        normalized = normalize_block_text(chunk)
        if not normalized or normalized in seen_blocks:
            continue
        seen_blocks.add(normalized)
        text_lines.append(normalized)
    text_plain = "\n\n".join(text_lines)

    flavor_blocks = wrapper.select("div.p-cards__detail-serif")
    flavor_lines: List[str] = []
    seen_flavor: set = set()
    for block in flavor_blocks:
        chunk = text_with_tokens(block)
        if not chunk:
            continue
        normalized = normalize_block_text(chunk)
        if not normalized or normalized in seen_flavor:
            continue
        seen_flavor.add(normalized)
        flavor_lines.append(normalized)
    flavor_plain = "\n\n".join(flavor_lines)

    copyright_tag = wrapper.find("p", class_="p-cards__detail-copyrights")
    if not copyright_tag:
        copyright_tag = wrapper.find("p", class_="p-cards__detail-copyright")
    copyright_text = text_from_tag(copyright_tag) or ""

    normalized_cardno = base_cardno(number or requested_cardno)
    if card_type == "Character" and soul is None:
        soul = 1
        print(f"Warning: defaulted soul to 1 for Character {normalized_cardno}", file=sys.stderr)
    if card_type in {"Event", "Climax"}:
        if power is None:
            power = 0
        if soul is None:
            soul = 0

    record = {
        "schema_version": SCHEMA_VERSION,
        "card_no": normalized_cardno,
        "name": name,
        "card_type": card_type,
        "rarity": rarity,
        "expansion_raw": expansion_raw,
        "traits": split_traits(traits_raw),
        "side": side,
        "color": color,
        "image_url": abs_url(image_src),
        "text": text_plain,
        "flavor": flavor_plain,
        "copyright": copyright_text,
        "source_url": DETAIL_URL_TEMPLATE.format(cardno=requested_cardno),
        "scraped_at": now_iso(),
        "level": level,
        "cost": cost,
        "power": power,
        "soul": soul,
        "triggers": triggers,
    }

    return record, None


async def fetch_text(
    client: httpx.AsyncClient, url: str, params: Optional[Dict[str, str]] = None
) -> httpx.Response:
    return await client.get(url, params=params)


async def fetch_with_retry(
    client: httpx.AsyncClient, url: str, params: Optional[Dict[str, str]] = None, label: str = ""
) -> httpx.Response:
    for attempt in range(3):
        try:
            resp = await fetch_text(client, url, params=params)
        except Exception:
            if attempt == 2:
                raise
            await asyncio.sleep(1 + attempt)
            continue
        if resp.status_code in (429, 500, 502, 503, 504):
            if attempt == 2:
                return resp
            await asyncio.sleep(1 + attempt)
            continue
        return resp
    return resp


async def recon_check(client: httpx.AsyncClient) -> Dict[str, Any]:
    result = {"listing_page_1_count": 0, "listing_page_1_url": "", "listing_page_9999_url": ""}
    resp = await fetch_with_retry(
        client, LISTING_ENDPOINT, params=listing_params(1), label="recon-page-1"
    )
    result["listing_page_1_url"] = str(resp.url)
    cardnos = parse_listing_cardnos(resp.text)
    result["listing_page_1_count"] = len(cardnos)

    resp_far = await fetch_with_retry(
        client, LISTING_ENDPOINT, params=listing_params(9999), label="recon-page-9999"
    )
    result["listing_page_9999_url"] = str(resp_far.url)
    result["listing_page_9999_count"] = len(parse_listing_cardnos(resp_far.text))
    return result


async def test_listing_parse(client: httpx.AsyncClient) -> None:
    resp = await fetch_with_retry(
        client, LISTING_ENDPOINT, params=listing_params(1), label="test-listing"
    )
    cardnos = parse_listing_cardnos(resp.text)
    if not cardnos:
        raise RuntimeError("Test A failed: no card numbers parsed from listing page 1")


async def test_detail_parse(client: httpx.AsyncClient) -> None:
    samples = [
        "GGO/SE50-E01",
        "GGO/SE50-E36",
        "GGO/SE50-E16",
    ]
    for cardno in samples:
        url = DETAIL_URL_TEMPLATE.format(cardno=cardno)
        resp = await fetch_with_retry(client, url, label=f"test-detail-{cardno}")
        if resp.status_code != 200:
            raise RuntimeError(f"Test B failed: {cardno} status {resp.status_code}")
        record, err = parse_detail_html(resp.text, cardno)
        if err or not record:
            raise RuntimeError(f"Test B failed: {cardno} parse error {err}")
        required = ["card_no", "name", "card_type", "expansion_raw", "image_url", "text"]
        for key in required:
            if not record.get(key):
                raise RuntimeError(f"Test B failed: {cardno} missing {key}")


async def test_idempotence(client: httpx.AsyncClient, out_dir: Path) -> None:
    resp = await fetch_with_retry(
        client, LISTING_ENDPOINT, params=listing_params(1), label="test-idempotence-listing"
    )
    cardnos = parse_listing_cardnos(resp.text)[:5]
    if len(cardnos) < 2:
        raise RuntimeError("Test C failed: not enough card numbers for idempotence test")

    import tempfile

    with tempfile.TemporaryDirectory(dir=out_dir) as tmp:
        tmp_dir = Path(tmp)
        ensure_out_dir(tmp_dir)
        done = set()
        sets_index: Dict[str, set] = defaultdict(set)

        await crawl_details(
            client,
            cardnos,
            done,
            title_map={},
            expansion_map={},
            out_dir=tmp_dir,
            sets_index=sets_index,
            concurrency=2,
        )
        first_lines = read_line_count(tmp_dir / "cards.jsonl")
        await crawl_details(
            client,
            cardnos,
            done,
            title_map={},
            expansion_map={},
            out_dir=tmp_dir,
            sets_index=sets_index,
            concurrency=2,
        )
        second_lines = read_line_count(tmp_dir / "cards.jsonl")
        if first_lines != second_lines:
            raise RuntimeError("Test C failed: idempotence check appended new lines")


def read_line_count(path: Path) -> int:
    if not path.exists():
        return 0
    with path.open("r", encoding="utf-8") as f:
        return sum(1 for _ in f)


async def discover_cards_for_params(
    client: httpx.AsyncClient,
    filters: Dict[str, str],
    label: str,
    failures_path: Optional[Path] = None,
    max_cards: Optional[int] = None,
    max_pages: Optional[int] = None,
) -> List[str]:
    page = 1
    seen: set = set()
    ordered: List[str] = []
    while True:
        if max_pages is not None and page > max_pages:
            break
        params = listing_params(page, **filters)
        try:
            resp = await fetch_with_retry(
                client, LISTING_ENDPOINT, params=params, label=f"discover-{label}-p{page}"
            )
        except Exception as exc:
            if failures_path:
                failure_obj = {
                    "card_no": "",
                    "source_url": str(httpx.URL(LISTING_ENDPOINT, params=params)),
                    "phase": "listing",
                    "error_type": "request_error",
                    "status_code": None,
                    "message": str(exc),
                    "scraped_at": now_iso(),
                }
                append_jsonl(failures_path, failure_obj)
            break
        if resp.status_code != 200:
            if failures_path:
                failure_obj = {
                    "card_no": "",
                    "source_url": str(resp.request.url),
                    "phase": "listing",
                    "error_type": f"http_{resp.status_code}",
                    "status_code": resp.status_code,
                    "message": f"status {resp.status_code}",
                    "scraped_at": now_iso(),
                }
                append_jsonl(failures_path, failure_obj)
            break
        if "/cardsearch_ex" not in str(resp.url):
            break
        cardnos = parse_listing_cardnos(resp.text)
        if not cardnos:
            break
        new_cardnos = [c for c in cardnos if c not in seen]
        if not new_cardnos:
            break
        for c in new_cardnos:
            seen.add(c)
            ordered.append(c)
        if max_cards is not None and len(seen) >= max_cards:
            break
        page += 1
    return ordered


async def discover_filter_group(
    client: httpx.AsyncClient,
    items: List[Tuple[str, str]],
    param_name: str,
    label_prefix: str,
    failures_path: Optional[Path],
    concurrency: int,
    max_cards: Optional[int] = None,
    max_pages: Optional[int] = None,
) -> Dict[str, List[str]]:
    results: Dict[str, List[str]] = {}
    semaphore = asyncio.Semaphore(concurrency)

    async def run_one(value: str) -> Tuple[str, List[str]]:
        async with semaphore:
            cardnos = await discover_cards_for_params(
                client,
                {param_name: value},
                label=f"{label_prefix}-{value}",
                failures_path=failures_path,
                max_cards=max_cards,
                max_pages=max_pages,
            )
            return value, cardnos

    tasks = [run_one(value) for value, _label in items]
    for coro in asyncio.as_completed(tasks):
        value, cardnos = await coro
        results[value] = cardnos
    return results


def append_jsonl(path: Path, obj: dict) -> None:
    with path.open("a", encoding="utf-8") as f:
        f.write(json.dumps(obj, ensure_ascii=False))
        f.write("\n")


def append_text_line(path: Path, text: str) -> None:
    with path.open("a", encoding="utf-8") as f:
        f.write(text)
        f.write("\n")


def load_existing(out_dir: Path) -> Tuple[set, Dict[str, set]]:
    done: set = set()
    sets_index: Dict[str, set] = defaultdict(set)
    done_file = out_dir / "done_cardnos.txt"
    cards_file = out_dir / "cards.jsonl"

    if done_file.exists():
        with done_file.open("r", encoding="utf-8") as f:
            for line in f:
                cardno = base_cardno(line.strip())
                if cardno:
                    done.add(cardno)

    if cards_file.exists():
        with cards_file.open("r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    obj = json.loads(line)
                except json.JSONDecodeError:
                    continue
                cardno = obj.get("card_no") or ""
                cardno = base_cardno(cardno)
                if cardno:
                    done.add(cardno)
                set_key = obj.get("set_key")
                if not set_key:
                    exp_val = obj.get("expansion_filter_value")
                    if exp_val:
                        set_key = str(exp_val)
                    else:
                        set_key = obj.get("expansion_raw") or ""
                if set_key and cardno:
                    sets_index[set_key].add(cardno)

    return done, sets_index


async def crawl_details(
    client: httpx.AsyncClient,
    cardnos: Sequence[str],
    done: set,
    title_map: Dict[str, str],
    expansion_map: Dict[str, str],
    out_dir: Path,
    sets_index: Dict[str, set],
    concurrency: int = 8,
) -> Tuple[int, int]:
    cards_path = out_dir / "cards.jsonl"
    done_path = out_dir / "done_cardnos.txt"
    failures_path = out_dir / "failures.jsonl"

    pending = [c for c in cardnos if c not in done]
    success = 0
    failures = 0

    async def fetch_one(cardno: str) -> Tuple[str, Optional[dict], Optional[str], Optional[int]]:
        url = DETAIL_URL_TEMPLATE.format(cardno=cardno)
        try:
            resp = await fetch_with_retry(client, url, label=f"detail-{cardno}")
        except Exception as exc:
            return cardno, None, f"request_error:{exc}", None
        if resp.status_code != 200:
            return cardno, None, f"http_{resp.status_code}", resp.status_code
        record, err = parse_detail_html(resp.text, cardno)
        if err:
            return cardno, None, err, resp.status_code
        return cardno, record, None, resp.status_code

    for i in range(0, len(pending), concurrency):
        batch = pending[i : i + concurrency]
        tasks = [fetch_one(cardno) for cardno in batch]
        results = await asyncio.gather(*tasks)
        for cardno, record, err, status in results:
            if err or not record:
                failures += 1
                failure_obj = {
                    "card_no": cardno,
                    "source_url": DETAIL_URL_TEMPLATE.format(cardno=cardno),
                    "phase": "detail",
                    "error_type": err or "unknown",
                    "status_code": status,
                    "message": err or "unknown",
                    "scraped_at": now_iso(),
                }
                append_jsonl(failures_path, failure_obj)
                continue

            base_no = base_cardno(record.get("card_no", cardno))
            if base_no in done:
                continue
            record["card_no"] = base_no
            record["title_filter_value"] = title_map.get(base_no)
            record["expansion_filter_value"] = expansion_map.get(base_no)
            set_key = record["expansion_filter_value"] or record.get("expansion_raw") or ""
            record["set_key"] = set_key

            append_jsonl(cards_path, record)
            append_text_line(done_path, base_no)
            done.add(base_no)
            success += 1
            if set_key:
                sets_index[set_key].add(base_no)

    return success, failures


def write_sorted_lines(path: Path, items: Iterable[str]) -> None:
    with path.open("w", encoding="utf-8") as f:
        for item in sorted(set(items)):
            f.write(item)
            f.write("\n")


def write_sets_index(path: Path, sets_index: Dict[str, set]) -> None:
    data = {k: sorted(v) for k, v in sorted(sets_index.items(), key=lambda x: x[0])}
    with path.open("w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)


def write_stats(path: Path, stats: dict) -> None:
    with path.open("w", encoding="utf-8") as f:
        json.dump(stats, f, ensure_ascii=False, indent=2)


def stable_sort_ids(pairs: List[Tuple[str, str]]) -> List[Tuple[str, str]]:
    return sorted(pairs, key=lambda p: (int(p[0]) if p[0].isdigit() else p[0]))


async def main() -> int:
    import argparse

    parser = argparse.ArgumentParser(description="Weiss Schwarz EN card scraper")
    parser.add_argument("--concurrency", type=int, default=8)
    parser.add_argument("--discovery-concurrency", type=int, default=4)
    parser.add_argument("--skip-tests", action="store_true")
    parser.add_argument(
        "--limit", type=int, default=0, help="Limit number of cards to crawl (debug)"
    )
    parser.add_argument(
        "--smoke",
        action="store_true",
        help="Fast check: limit to 1000 cards and skip set enumeration",
    )
    parser.add_argument(
        "--max-discovery-pages", type=int, default=0, help="Cap pages per discovery query"
    )
    args = parser.parse_args()

    out_dir = Path("out")
    ensure_out_dir(out_dir)
    failures_path = out_dir / "failures.jsonl"

    done, sets_index = load_existing(out_dir)

    max_conn = max(args.concurrency, args.discovery_concurrency)
    limits = httpx.Limits(max_connections=max_conn * 2, max_keepalive_connections=max_conn)
    timeout = httpx.Timeout(30.0, connect=10.0)
    headers = {"User-Agent": USER_AGENT}

    if args.smoke and not args.limit:
        args.limit = 1000

    discovery_max_pages = args.max_discovery_pages or None

    async with httpx.AsyncClient(
        limits=limits, timeout=timeout, headers=headers, follow_redirects=True
    ) as client:
        print("Phase 0: recon")
        recon = await recon_check(client)
        print(
            f"  listing page 1 count: {recon.get('listing_page_1_count')}, page 9999 url: {recon.get('listing_page_9999_url')}"
        )

        if not args.skip_tests:
            print("Phase 5: tests A-C")
            await test_listing_parse(client)
            await test_detail_parse(client)
            await test_idempotence(client, out_dir)
            print("  tests OK")

        filters_html_resp = await fetch_with_retry(client, CARDLIST_URL, label="filters")
        filter_options = extract_select_options(filters_html_resp.text)

        title_options = stable_sort_ids(filter_options.get("title", []))
        expansion_options = stable_sort_ids(filter_options.get("expansion_name", []))
        category_options = stable_sort_ids(filter_options.get("category", []))

        discovered_set: set = set()
        title_map: Dict[str, set] = defaultdict(set)
        expansion_map: Dict[str, set] = defaultdict(set)

        print("Phase 3: discovery")
        print(
            f"  filters: title={len(title_options)}, expansion={len(expansion_options)}, category={len(category_options)}"
        )

        # Global discovery
        global_cardnos = await discover_cards_for_params(
            client,
            {},
            label="global",
            failures_path=failures_path,
            max_cards=args.limit or None,
            max_pages=discovery_max_pages,
        )
        discovered_set.update(global_cardnos)
        print(f"  global discovery: {len(global_cardnos)} cardnos")

        if not args.smoke:
            # Expansion discovery
            expansion_results = await discover_filter_group(
                client,
                expansion_options,
                param_name="expansion_name",
                label_prefix="expansion",
                failures_path=failures_path,
                concurrency=args.discovery_concurrency,
                max_pages=discovery_max_pages,
            )
            for value, cardnos in expansion_results.items():
                for c in cardnos:
                    expansion_map[c].add(value)
                discovered_set.update(cardnos)
            print(f"  expansion discovery done: {len(expansion_results)} filters")

            # Title discovery
            title_results = await discover_filter_group(
                client,
                title_options,
                param_name="title",
                label_prefix="title",
                failures_path=failures_path,
                concurrency=args.discovery_concurrency,
                max_pages=discovery_max_pages,
            )
            for value, cardnos in title_results.items():
                for c in cardnos:
                    title_map[c].add(value)
                discovered_set.update(cardnos)
            print(f"  title discovery done: {len(title_results)} filters")

            # Category discovery
            category_results = await discover_filter_group(
                client,
                category_options,
                param_name="category",
                label_prefix="category",
                failures_path=failures_path,
                concurrency=args.discovery_concurrency,
                max_pages=discovery_max_pages,
            )
            for _value, cardnos in category_results.items():
                discovered_set.update(cardnos)
            print(f"  category discovery done: {len(category_results)} filters")
        else:
            print("  smoke mode: skipping per-set discovery")

        cardnos_all = sorted(discovered_set)
        if args.limit:
            cardnos_all = cardnos_all[: args.limit]

        write_sorted_lines(out_dir / "cardnos_all.txt", cardnos_all)
        print(f"  total discovered: {len(cardnos_all)}")

        # Normalize mappings to a single value per card (stable smallest)
        title_map_final = {k: sorted(v)[0] for k, v in title_map.items() if v}
        expansion_map_final = {k: sorted(v)[0] for k, v in expansion_map.items() if v}

        pending_count = len([c for c in cardnos_all if c not in done])
        print(f"Phase 4: detail crawl (pending {pending_count})")
        success, failures = await crawl_details(
            client,
            cardnos_all,
            done,
            title_map=title_map_final,
            expansion_map=expansion_map_final,
            out_dir=out_dir,
            sets_index=sets_index,
            concurrency=args.concurrency,
        )
        print(f"  detail crawl done: success={success}, failures={failures}")

    # Write sets index and done cardnos
    write_sets_index(out_dir / "sets_index.json", sets_index)
    write_sorted_lines(out_dir / "done_cardnos.txt", done)

    failure_count = read_line_count(out_dir / "failures.jsonl")

    done_in_discovery = len(set(cardnos_all) & set(done))
    stats = {
        "schema_version": SCHEMA_VERSION,
        "scraped_at": now_iso(),
        "recon": recon,
        "discovered_count": len(cardnos_all),
        "done_count": len(done),
        "new_success_count": success,
        "failure_count": failure_count,
        "filters": {
            "title_count": len(title_options),
            "expansion_count": len(expansion_options),
            "category_count": len(category_options),
        },
        "sanity": {
            "success_plus_failure_equals_discovered": (done_in_discovery + failure_count)
            == len(cardnos_all),
        },
    }
    write_stats(out_dir / "stats.json", stats)

    return 0


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
