from PIL import Image as pil_image
from keras.preprocessing import image
from keras.applications.inception_resnet_v2 import InceptionResNetV2, preprocess_input, decode_predictions
import numpy as np

from io import BytesIO
import os
import sys
import socket
import json


def start_server():
    with open('keyword_mapping.json', 'r') as f:
        kw_mapping = json.load(f)

    model = InceptionResNetV2()
    sock_path = "imclassif.sock"
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
                size = int.from_bytes(conn.recv(4), byteorder='big')
                print('Received a request to classify image of size ' + str(size))

                img = conn.recv(size, socket.MSG_WAITALL)
                try:
                    result = classify(model, kw_mapping, img)
                except:
                    result = ''

                result_bytes = str.encode(result)
                conn.send(len(result_bytes).to_bytes(4, byteorder='big'))
                conn.send(result_bytes)
        finally:
            conn.close()


def classify(model, kw_mapping, img_bytes):
    img = pil_image.open(BytesIO(img_bytes))
    if img.mode != 'RGB':
        img = img.convert('RGB')
    img = img.resize((299, 299), pil_image.NEAREST)
    x = image.img_to_array(img)
    x = np.expand_dims(x, axis=0)
    x = preprocess_input(x)
    preds = model.predict(x)
    keywords = [pred[1] for pred in decode_predictions(preds, top=3)[0]]
    terms = ';'.join(','.join(kw_mapping[k]) for k in keywords)

    return terms


start_server()
