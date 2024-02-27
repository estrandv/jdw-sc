# Simple script for testing the note_on_timed message against the main application

from pythonosc import udp_client
import time

# Hardcoded default port of jdw-sc main application

#client = udp_client.SimpleUDPClient("127.0.0.1", 13339) # Via router (requires working subscription)
client = udp_client.SimpleUDPClient("127.0.0.1", 13331) # Straight to main application
client.send_message("/test", [1, "A string", 1337.0, "/try_this", "whoah"])

# This creates a ringing first tone
client.send_message("/note_on_timed", [
    "gentle", # SynthDef to use, See scd/synths/brute.scd
    "brute_TEST_HOLD", # Arbitrary unique external id for the ringing note
    "6.0", # Gate time ("note off after n sec")
    "freq", # Named args continue from here
    355.0,
    "attT",
    3.0,
    "relT",
    3.0,
    "susL",
    0.7,
    "lfoD",
    1.0
])

time.sleep(0.5)

arp = [440.0, 560.0, 138.3, 220.0, 440.0, 588.8, 220.0]
i = 0

# Loop the arp
for _ in range(0,14):

    if i > (len(arp) - 1):
        i = 0

    client.send_message("/note_on_timed", [
        "brute",
        "brute_TEST" + str(i),
        "0.04",
        "freq",
        arp[i] * 0.8,
        "attT",
        0.08,
        "relT",
        1.6,
        "fx",
        0.0 + ( i * 0.01 )
    ])
    time.sleep(0.2 + (i * 0.05))
    i+=1

for i in range(0, 53):
    client.send_message("/note_on_timed", [
        "brute",
        "brute_TEST" + str(i),
        "0.04",
        "freq",
        80.0 + (i * 22.0),
        "fx",
        0.03,
        "relT",
        2.2,
        "amp",
        0.4
    ])

    time.sleep(0.02)