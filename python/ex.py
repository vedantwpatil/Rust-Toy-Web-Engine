import ssl
import socket


class Url:
    def __init__(self, url: str):
        self.scheme, url = url.split("://", 1)
        assert self.scheme in ["http", "https"]

        if self.scheme == "http":
            self.port = 80
        elif self.scheme == "https":
            self.port = 443

        if "/" not in url:
            url = url + "/"
        self.host, url = url.split("/", 1)
        self.path = "/" + url

        if ":" in self.host:
            self.host, self.port = self.host.split(":", 1)
            self.port = int(self.port)

    def request(self):
        s = socket.socket(
            family=socket.AF_INET, type=socket.SOCK_STREAM, proto=socket.IPPROTO_TCP
        )
        s.connect((self.host, self.port))
        if self.scheme == "https":
            ctx = ssl.create_default_context()
            s = ctx.wrap_socket(s, server_hostname=self.host)

        request = "GET {} HTTP/1.0\r\n".format(self.path)
        request += "Host: {}\r\n".format(self.host)
        request += "\r\n"
        s.send(request.encode("utf8"))

        # Returns a file like object that contains all the bytes we receive from the server
        # We then take all those bytes and then convert them into a string with the utf encoding
        response = s.makefile("r", encoding="utf8", newline="\r\n")

        # You could check that the server is talking in http 1.0 however there are a lot of servers which contain outdated error codes
        statusLine = response.readline()
        version, status, explanation = statusLine.split(" ", 2)

        response_headers = {}
        while True:
            line = response.readline()
            if line == "\r\n":
                break
            header, value = line.split(":", 1)
            # Populate the map of header names to header values forcing all them to be lowercase since they typically are case sensitive
            response_headers[header.casefold()] = value.strip()

        # print(f"{response}\n")
        # for key, value in response_headers.items():
        #     print(f"Key: {key}\nValue: {value}")


def show(body):
    in_tag = False

    for c in body:
        if c == "<":
            in_tag = True
        elif c == ">":
            in_tag = False
        elif not in_tag:
            print(c, end="")


def load(url):
    body = url.request()
    show(body)


def test():
    url = Url("http://www.google.com/")
    url.request()


test()
