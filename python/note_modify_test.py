# Test modification of running notes with external ids matching arbitrary regex:es

import time
from pythonosc import udp_client

# Hardcoded default port of jdw-sc main application
client = udp_client.SimpleUDPClient("127.0.0.1", 13331) # Straight to main application


# Create a synthdef to use 
with open("synths/example.scd", "r") as synthdef:
    client.send_message("/create_synthdef", synthdef.read())


# See better explanation of note on parameters in note_on_timed_test.py
client.send_message("/note_on", [
    "example",
    "brute_MODTEST_1",
    0,
    "freq",
    444.0,
    "relT",
    0.2,
    "prt",
    2.5,
    "lfoS",
    182.2,
    "lfoD",
    0.5
])

time.sleep(0.5)

client.send_message("/note_on", [
    "example",
    "brute_MODTEST_2",
    0,
    "freq",
    128.2,
    "prt",
    0.1
])


time.sleep(1.0)

client.send_message("/note_modify", [
    "brute_MODTEST_1", # External id regex - here an exact match for our id
    0,
    "freq", # Args, same as any note_on message
    380.0,
    "lfoS",
    44.4,
    "amp",
    0.9
])

time.sleep(1.0)

# Fun with loops
for i in range(0, 40):
    time.sleep(0.003 * i)

    client.send_message("/note_modify", [
        "brute_MODTEST_2",
        0,
        "freq",
        128.2 + (i * 22.0),
        "amp",
        1.0 - (i * 0.015)
    ])

time.sleep(2.0)

client.send_message("/note_modify", [
    "brute_MODTEST_(.*)", # Here we use a wildcard regex to catch both our created notes
    0,
    "gate",
    0.0
])

time.sleep(1.0)

# This should not trigger since the notes should be cleaned after the gate=0
# Cleanup is in a bit of a flux - this can be a bit shaky
client.send_message("/note_modify", [
    "brute_MODTEST_(.*)",
    0,
    "gate",
    1.0
])