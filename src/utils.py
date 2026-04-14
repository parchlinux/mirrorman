import urllib.request
from urllib.error import URLError

def check_url(url):
    try:
        with urllib.request.urlopen(url, timeout=5) as response:
            return response.getcode() == 200
    except URLError:
        return False
