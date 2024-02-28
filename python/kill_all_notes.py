from pythonosc import udp_client

# Send a match-all wildcard to turn all running notes off using gate=0
client = udp_client.SimpleUDPClient("127.0.0.1", 13331)
client.send_message("/note_modify", [
    "(.*)",
    0,
    "gate",
    0.0
])