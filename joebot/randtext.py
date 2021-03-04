import markovify

from io import BytesIO
import os
import socket
import json

# Usage: git clone https://github.com/quest-prophets/Quest-Snickers-bot
# cd prep
# mkdir text
# copy .txts there
# ./generate_model
# copy model.json to randmodel.json


def start_server():
    with open('randmodel.json', 'r') as f:
        model_json = f.read()

    model = markovify.Text.from_json(model_json)

    sock_path = "randtext.sock"
    try:
        os.unlink(sock_path)
    except OSError:
        pass

    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.bind(sock_path)
    s.listen()

    print('Listening on socket ' + sock_path)

    while True:
        conn, _ = s.accept()
        try:
            while True:
                max_text_len = int.from_bytes(conn.recv(4), byteorder='big')
                if max_text_len > 1:
                    print('Received a request to generate random text')

                    text = None
                    while text is None:
                        text = model.make_short_sentence(max_text_len)

                    result_bytes = str.encode(text)
                    conn.send(len(result_bytes).to_bytes(4, byteorder='big'))
                    conn.send(result_bytes)
        finally:
            conn.close()


start_server()
