#!/usr/bin/env python

import asyncio
import json
import sys
from threading import Thread

from websockets import connect
from websockets.exceptions import ConnectionClosedError

__data = None
websocket = None


async def listen(uri):
    global websocket
    async with connect(uri) as websocket:
        try:
            while True:
                data = await websocket.recv()
                if type(data) is str:
                    handle(json.loads(data))
                else:
                    raise Exception("Got binary data")
        except ConnectionClosedError:
            print("ConnectionClosedError")
        except Exception as e:
            print("Unknown error!!", type(e), e)


def get_data():
    return __data


def handle(protocol: dict):
    global __data
    print("Got:", protocol)
    if "DataLocation" in protocol:
        with open(protocol["DataLocation"], "rb") as fo:
            data = fo.read()
            print("Got", len(data), "bytes of data")
            __data = data


def listen_thread():
    asyncio.run(listen("ws://localhost:9005"))

started = False

def start():
    global started, thread
    if started:
        return
    started = True
    thread = Thread(target=listen_thread)
    thread.start()
    

def register():
    pass

def unregister():
    global websocket
    if websocket is not None:
        loop = asyncio.get_event_loop()
        loop.run_until_complete(websocket.close())
        loop.close()
        websocket = None

if __name__ == "__main__" and not sys.flags.interactive:
    start()
