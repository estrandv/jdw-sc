from pythonosc import udp_client
import config

# Send a match-all wildcard to turn all running notes off using gate=0
client = udp_client.SimpleUDPClient(config.HOST, config.PORT)
client.send_message("/note_modify", [
    "(.*)",
    0,
    "gate",
    0.0
])