import random
import time
import uuid
from typing import Optional

import requests as rq


# settings
N_REQUESTS = 10_000
ENDPOINT = "http://localhost:3000/api/shorten"
OUTPUT1 = "data_aliases.txt"
OUTPUT2 = "data_urls.txt"

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

    segments = []
    for _ in range(random.randint(1, 4)):
        w = random.choice(RANDOM_PATH_WORDS)
        segments.append(w)

    # sometimes include uuid segment
    if random.random() < 0.25:
        segments.append(uuid.uuid4().hex[:12])

    path = "/" + "/".join(segments)

    url = f"{scheme}://{host}{path}"

    # sometimes include query
    if random.random() < P_INCLUDE_QUERY:
        q = {
            "ref": random.choice(["a", "b", "c", "newsletter", "social"]),
            "id": str(random.randint(1, 1_000_000)),
        }
        url += f"?ref={q['ref']}&id={q['id']}"

    return url

def shorten_request(session: rq.Session, payload: dict) -> Optional[dict]:
    r = session.post(ENDPOINT, json=payload, timeout=5)

    assert r.status_code == 201, "request failed"
    
    return r.json()


def main() -> None:
    with rq.Session() as session, open(OUTPUT1, "w", encoding="utf-8") as out1, open(OUTPUT2, "w", encoding="utf-8") as out2:
        for n in range(N_REQUESTS):
            random_url = make_random_url()
            out2.write(random_url + "\n")

            payload = { "url": random_url }

            response = shorten_request(session, payload)
            alias = response.get("alias")

            out1.write(alias + "\n")

            if (n + 1) % 25 == 0:
                print(f"processed {n + 1}/{N_REQUESTS}")
            
            time.sleep(0.02)

    print(f"done")


if __name__ == "__main__":
    main()
