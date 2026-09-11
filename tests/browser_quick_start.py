"""Exercise the built README consumer through its real HTML/JS/WASM loader."""
import functools
import http.server
import pathlib
import threading
import unittest

from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.support.ui import WebDriverWait


class QuickStart(unittest.TestCase):
    def test_loader_mounts_ui_and_callback_updates_dom(self):
        root = pathlib.Path(__file__).resolve().parents[1] / "test-slint-dom-user"
        handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(root))
        server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        options = webdriver.ChromeOptions()
        options.add_argument("--headless=new")
        options.add_argument("--no-sandbox")
        options.add_argument("--disable-dev-shm-usage")
        try:
            with webdriver.Chrome(options=options) as browser:
                browser.get(f"http://127.0.0.1:{server.server_port}/")
                wait = WebDriverWait(browser, 30)
                wait.until(lambda driver: driver.find_elements(By.CSS_SELECTOR, ".sd-component"))
                status = browser.find_element(By.CSS_SELECTOR, ".sd-component [role=status]")
                self.assertEqual(status.text, "Ready")
                self.assertFalse(browser.find_elements(By.ID, "loading"))
                browser.find_element(By.CSS_SELECTOR, ".sd-component button").click()
                wait.until(lambda _: status.text == "Running")
        finally:
            server.shutdown()
            server.server_close()
            thread.join()


if __name__ == "__main__":
    unittest.main()
