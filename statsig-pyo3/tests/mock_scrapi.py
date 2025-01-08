from pytest_httpserver import HTTPServer
from werkzeug import Response, Request
import threading
import json
import gzip
import time


class MockScrapi:
    def __init__(self, httpserver: HTTPServer):
        self.httpserver = httpserver
        self.requests = []
        self.logged_events = []
        self.lock = threading.Lock()

    def reset(self):
        with self.lock:
            self.requests.clear()
            self.logged_events = []

    def stub(self, endpoint, response="", status=200, method="GET"):
        def handler(req: Request):
            with self.lock:
                self.requests.append(req)
                if "/v1/log_event" in req.path:
                    data = req.get_data()
                    json_str = gzip.decompress(data)
                    req_json = json.loads(json_str)
                    self.logged_events.extend(req_json["events"])

            return Response(response, status=status)

        self.httpserver.expect_request(endpoint, method=method).respond_with_handler(
            handler
        )

    def url_for_endpoint(self, endpoint):
        return self.httpserver.url_for(endpoint)

    def times_called_for_endpoint(self, endpoint):
        with self.lock:
            return sum(1 for req in self.requests if endpoint in req.path)

    def get_logged_event_count(self):
        with self.lock:
            return len(self.logged_events)

    def get_logged_events(self, include_diagnostics=False):
        with self.lock:
            if include_diagnostics:
                return self.logged_events
            else:
                return [
                    event
                    for event in self.logged_events
                    if event.get("eventName") != "statsig::diagnostics"
                ]

    def get_requests(self):
        with self.lock:
            return list(self.requests)

    def get_requests_for_endpoint(self, endpoint):
        with self.lock:
            return [req for req in self.requests if endpoint in req.path]
