from dataclasses import dataclass
import random
import secrets
import string
from typing import Any

from locust import FastHttpUser, task, between, constant_throughput


ALIASES = []
with open("data_aliases.txt", "r") as f:
    ALIASES = [l for line in f if (l := line.strip())]

URLS = []
with open("data_urls.txt", "r") as f:
    URLS = [l for line in f if (l := line.strip())]


def random_number(low: int = 0, high: int = 100) -> int:
    return random.randint(low, high)


def random_string(length: int) -> str:
    ALPHANUM = string.ascii_letters + string.digits
    return "".join(secrets.choice(ALPHANUM) for _ in range(length))


def random_url() -> str:
    return f"https://example.com/{random_string(10)}?q={random_string(6)}"


def sample_url_uniform():
    return random.choice(URLS)


def sample_top20_biased(p_top=0.8):
    n = len(ALIASES)
    top_n = max(1, n // 5)
    if random.random() < p_top:
        return ALIASES[random.randrange(top_n)]
    return random.choice(ALIASES)

def _as_str(x: Any) -> str | None:
    return x if isinstance(x, str) else None

@dataclass(frozen=True)
class LinkItem:
    alias: str
    url: str

def _as_link_list(js: Any) -> list[LinkItem] | None:
    if not isinstance(js, list):
        return None

    out: list[LinkItem] = []
    for i, item in enumerate(js):
        if not isinstance(item, dict):
            return None
        alias = _as_str(item.get("alias"))
        url = _as_str(item.get("url"))
        if alias is None or url is None:
            return None
        out.append(LinkItem(alias=alias, url=url))

    return out


class BaseUser(FastHttpUser):
    abstract = True

    def _restore_session(self) -> bool:
        with self.rest("GET", "/api/auth/me") as resp:
            if resp.status_code == 200:
                return True
            # treat 401 as success
            elif resp.status_code == 401:
                resp.success()
                return False
            else:
                resp.failure(f"restore session code: {resp.status_code}")
                return False

    def _authenticate(self, username, password, register=False) -> bool:
        body = {"username": username, "password": password}
        action = "register" if register else "login"
        with self.rest("POST", f"/api/auth/{action}", json=body) as resp:
            if resp.status_code != 200:
                resp.failure(f"failed to create an account {resp.status_code} {resp.text}")
                return False
        return True
    
    def _shorten(self, url: str, name=None, password=None) -> str | None:
        body = {"url": url}
        if name is not None:
            body["name"] = name
        if password is not None:
            body["password"] = password

        with self.rest("POST", "/api/shorten", json=body) as resp:
            if resp.js is None:
                return None
            elif "alias" not in resp.js:
                resp.failure(f"'alias' missing from response {resp.text}")
                return None
            return resp.js.get("alias")
    
    def _list_user_links(self) -> list[LinkItem] | None:
        with self.rest("GET", "/api/user/list") as resp:
            if resp.status_code != 200:
                resp.failure(f"list failed: {resp.status_code} {resp.text}")
                return None

            if resp.js is None:
                resp.failure(f"list non-json: {resp.text}")
                return None

            links = _as_link_list(resp.js)
            if links is None:
                resp.failure(f"list invalid shape (expected list[{{alias,url}}]): {resp.text}")
                return None

            return links
        
    def _delete_link(self, alias: str) -> bool:
        with self.rest("DELETE", f"/api/user/link/{alias}", name="/api/user/link") as resp:
            if resp.status_code != 204:
                resp.failure(f"delete failed: {resp.status_code} {resp.text}")
                return False
            return True

    def _logout(self) -> bool:
        with self.rest("POST", "/api/user/logout") as resp:
            if resp.status_code != 204:
                resp.failure(f"logout failed: {resp.status_code} {resp.text}")
                return False
            return True


class CoreUser(BaseUser):
    weight = 800
    wait_time = constant_throughput(1)

    @task(80)
    def normal_redirect(self):
        alias = sample_top20_biased()
        with self.rest("GET", f"/r/{alias}", allow_redirects=False, name="/r/") as resp:
            if resp.status_code != 307:
                resp.failure(f"expected 307, got {resp.status_code}")
            elif "Location" not in resp.headers:
                resp.failure("307 missing location header")

    @task(20)
    def shorten_link(self):
        url = sample_url_uniform()
        self._shorten(url)



class AuthUser(BaseUser):
    weight = 180
    wait_time = between(5,15)

    def on_start(self):
        self.username = f"{random_string(10)}@test.local"
        self.password = random_string(16)
        self.account = False

    @task
    def user_flow(self):
        # we'll try to restore session here, in case the flow was interrupted
        if not self._restore_session():
            if not self.account:
                if not self._authenticate(self.username, self.password, register=True):
                    return
                else:
                    self.account = True
            else:
                self._authenticate(self.username, self.password)

        for _ in range(0, random_number(1, 10)):
            url = random_url()
            alias = self._shorten(url)
            if alias is None:
                return
        
        user_links = self._list_user_links()
        if user_links is None:
            return
        
        for link in user_links:
            self._delete_link(link.alias)

        self._logout()


class UnlockUser(BaseUser):
    weight = 20
    wait_time = between(5, 15)

    @task
    def protected_links_flow(self):
        url = sample_url_uniform()
        alias = None
        password = random_string(16)

        # create protected link
        alias = self._shorten(url, password=password)
        if alias is None:
            return
        
        # trigger unlock prompt
        with self.rest("GET", f"/r/{alias}", allow_redirects=False, name="/r/") as resp:
            if resp.status_code != 307:
                resp.failure(f"expected 307, got {resp.status_code}")
        
            unlock_loc = resp.headers.get("Location")
            if not unlock_loc:
                resp.failure("missing Location header on 307 response")
                return

            expected = f"/unlock/{alias}"
            if unlock_loc != expected:
                resp.failure(f"expected Location {expected} got {unlock_loc}")
        
        # unlock the protected link
        with self.rest("POST", f"/api/unlock/{alias}", json={"password": password}, name="/api/unlock/") as resp:
            if resp.status_code != 200:
                resp.failure(f"expected 200, got {resp.status_code}")
            
            if resp.js.get("url") != url:
                resp.failure(f"did not get matching url")
