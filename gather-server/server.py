#!/usr/bin/env python3

from aiohttp import web, web_request

valid_client_keys = [
    "test_key",
]

seeds_to_search = [f"test{x}" for x in range(500)]


async def assign_handler(request: web_request.Request):
    global valid_client_keys

    if "client_key" not in request.match_info:
        return web.Response(status=401, text="Missing client key")

    if request.match_info["client_key"] not in valid_client_keys:
        return web.Response(status=401, text="Invalid client key")

    count = int(request.match_info.get("count", "1"))

    seeds = "\n".join([seeds_to_search.pop() for _ in range(count)])

    return web.Response(status=200, text=seeds)


async def submit_handler(request: web_request.Request):
    global valid_client_keys

    if "client_key" not in request.match_info:
        return web.Response(status=401, text="Missing client key")

    if request.match_info["client_key"] not in valid_client_keys:
        return web.Response(status=401, text="Invalid client key")

    print(
        f"Received submission for seed {request.headers['SpicyGarden-Seed']}")
    print(await request.text())

    return web.Response(status=204)


server = web.Application()

server.add_routes([
    web.get("/assign_seeds/{client_key}/{count}", assign_handler),
    web.post("/submit_result/{client_key}", submit_handler),
])

if __name__ == "__main__":
    web.run_app(server)
