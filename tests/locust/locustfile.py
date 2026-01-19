import random

from locust import FastHttpUser, task, constant_throughput

ALIASES = []
with open("data_aliases.txt", "r") as f:
    ALIASES = [l for line in f if (l := line.strip())]

URLS = []
with open("data_urls.txt", "r") as f:
    URLS = [l for line in f if (l := line.strip())]

def sample_url_uniform():
    return random.choice(URLS)

def sample_top20_biased(p_top=0.8):
    n = len(ALIASES)
    top_n = max(1, n // 5)
    if random.random() < p_top:
        return ALIASES[random.randrange(top_n)]
    return random.choice(ALIASES)

# 80% GET 20% POST
class MyUser(FastHttpUser):
    wait_time = constant_throughput(1)

    @task(4)
    def get_url(self):
        alias = sample_top20_biased()
        with self.client.get(
            f"/r/{alias}",
            name="GET redirect",
            allow_redirects=False,
            catch_response=True,
        ) as r:
            if r.status_code != 308:
                r.failure(f"expected 308, got {r.status_code}")
            elif "Location" not in r.headers:
                r.failure("308 missing location header")
            else:
                r.success()

    @task(1)
    def post_shorten(self):
        url = sample_url_uniform()
        with self.client.post(
            "/api/shorten",
            name="POST shorten",
            json={ "url": url },
            catch_response=True,
        ) as r:
            if r.status_code != 201:
                body_snip = (r.text or "")[:200]
                r.failure(f"expected 201, got {r.status_code}. body={body_snip!r}")
            else:
                r.success()