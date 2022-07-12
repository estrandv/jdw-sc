# Test modification of running notes with external ids matching arbitrary regex:es

import time
from pythonosc import udp_client

# Hardcoded default port of jdw-sc main application
client = udp_client.SimpleUDPClient("127.0.0.1", 13331) # Straight to main application

# See better explanation of note on parameters in note_on_timed_test.py
client.send_message("/note_on", [
    "miniBrute",
    "miniBrute_MODTEST_1",
    "freq",
    444.0,
    "relT",
    0.6,
    "prt",
    2.5,
    "lfoS",
    182.2,
    "lfoD",
    0.5
])

time.sleep(0.5)

client.send_message("/note_on", [
    "miniBrute",
    "miniBrute_MODTEST_2",
    "freq",
    128.2,
    "prt",
    1.0
])


time.sleep(1.0)

client.send_message("/note_modify", [
    "miniBrute_MODTEST_1", # External id regex - here an exact match for our id
    "freq", # Args, same as any note_on message
    380.0,
    "lfoS",
    44.4
])

time.sleep(1.0)

client.send_message("/note_modify", [
    "miniBrute_MODTEST_2",
    "freq",
    180.0,
    "lfoD",
    0.8,
    "lfoS",
    0.1
])

time.sleep(3.0)

client.send_message("/note_modify", [
    "miniBrute_MODTEST_(.*)", # Here we use a wildcard regex to catch both our created notes
    "gate",
    0
])

time.sleep(1.0)

# This should not trigger since the notes should be cleaned after the gate=0
# Cleanup is in a bit of a flux - this can be a bit shaky
client.send_message("/note_modify", [
    "miniBrute_MODTEST_(.*)",
    "gate",
    1
])