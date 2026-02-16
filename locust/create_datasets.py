import random
import uuid
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import Tuple

import requests as rq


# settings
N_REQUESTS = 10_000
ENDPOINT = "http://localhost:3000/api/shorten"
OUTPUT1 = "data_aliases.txt"
OUTPUT2 = "data_urls.txt"
MAX_WORKERS = 40
TIMEOUT_S = 5


# random url settings
RANDOM_URL_SCHEMES = ["https", "http"]
RANDOM_URL_BASES = [
    "example.com",
    "news.ycombinator.com",
    "en.wikipedia.org",
    "github.com",
    "stackoverflow.com",
]
RANDOM_PATH_WORDS = [
    "alpha", "beta", "gamma", "delta", "docs", "blog", "post", "item", "api", "v1",
    "how-to", "guide", "tips", "notes", "release", "changelog",
]
P_INCLUDE_QUERY = 0.6


def make_random_url() -> str:
    scheme = random.choice(RANDOM_URL_SCHEMES)
    host = random.choice(RANDOM_URL_BASES)

    segments = [random.choice(RANDOM_PATH_WORDS) for _ in range(random.randint(1, 4))]
    if random.random() < 0.25:
        segments.append(uuid.uuid4().hex[:12])

    url = f"{scheme}://{host}/" + "/".join(segments)

    if random.random() < P_INCLUDE_QUERY:
        ref = random.choice(["a", "b", "c", "newsletter", "social"])
        _id = random.randint(1, 1_000_000)
        url += f"?ref={ref}&id={_id}"

    return url


def shorten_url(url: str) -> str:
    with rq.Session() as session:
        r = session.post(ENDPOINT, json={"url": url}, timeout=TIMEOUT_S)

    if r.status_code != 201:
        raise RuntimeError(f"HTTP {r.status_code} for {url}: {r.text[:200]}")

    data = r.json()
    alias = data.get("alias")
    if not alias:
        raise RuntimeError(f"missing alias for {url}: {data}")
    return alias


def worker(i: int) -> Tuple[int, str, str]:
    url = make_random_url()
    alias = shorten_url(url)
    return i, url, alias


def main() -> None:
    results_url = [""] * N_REQUESTS
    results_alias = [""] * N_REQUESTS

    processed = 0
    futures = []

    with ThreadPoolExecutor(max_workers=MAX_WORKERS) as ex:
        futures = [ex.submit(worker, i) for i in range(N_REQUESTS)]

        try:
            for fut in as_completed(futures):
                i, url, alias = fut.result()
                results_url[i] = url
                results_alias[i] = alias

                processed += 1
                if processed % 25 == 0:
                    print(f"processed {processed}/{N_REQUESTS}")

        except Exception:
            # cancel impending futures
            for f in futures:
                f.cancel()
            raise

    with open(OUTPUT2, "w", encoding="utf-8") as out2:
        out2.write("\n".join(results_url) + "\n")
    with open(OUTPUT1, "w", encoding="utf-8") as out1:
        out1.write("\n".join(results_alias) + "\n")

    print("done")


if __name__ == "__main__":
    main()
