# Джобот

Discord chat games based on _your_ VKontakte mesage history, featuring:
* _Taki_ ([Polish](https://en.wiktionary.org/wiki/taki#Polish) for _such_) — a guessing game
for people who think they know each other well :^)
* _Joker_ — _Impact_ful philosophical ruminations on society and life
* _Mashup_ — a Markov chain generator with time- and source-based selection of inputs
* _WDYT_ — klook up your past thoughts about anything (well, anything you've written before, at least...)
* _Img2msg_ — _WDYT_

Plus some utilities to avoid having third-party bots in the chatroom:
* _Poll_, a _Simple Poll_ clone

## Acknowledgements

_WDYT_ and _Joker_ functionality is ported from [sunDalik/vk-bot](https://github.com/sunDalik/vk-bot).

## Legacy versions

* [Telegram](https://github.com/texbois/joebot/tree/telegram/) (only featuring _Taki_, but much more lightweight
thanks to a handwritten API client)

## Building the bot

Start by exporting a Vkontakte chat you'd like to have as the base for the chat games.
Use [VkOpt](https://chrome.google.com/webstore/detail/vkopt/hoboppgpbgclpfnjfdidokiilachfcbb)
(be sure to select the _Export as .html_ option).

The exported message dump should be named `messages.html` and placed in the `joebot` crate root.

Next, in the `joebot` crate root:
1. Create a `chain_sources.json` file listing the sources for the textual Markov chain, for example:
```json
[
  { "type": "MessageDump", "path": "path/to/vkopt/message/dump.html" },
  { "type": "Text", "path": "книга.txt", "names": ["книга"], "year": 2017, "day": 200 },
]
```
2. Run `cargo run --release --example mkbin` to covert the specified sources to chain data.

## Getting up & running

1. Deploy the following files:
* `target/release/joebot`
* `chain.bin`
* `messages.html`
* `imclassif.py`
* `keyword_mapping.json`

2. Create a `config.json` file with the following contents:
```json
{
  // The bot only responds to messages from this channel:
  "channel_id": 0000,
  // Only users with the short names defined below
  // are included in chat games:
  "user_matcher": {
    "short_name": "regex|to|match|name"
  },
  "user_penalties": {
    // Penalized users appear less frequently in Taki games
    // Max penalty = number of users, removes the user from Taki entirely
    // (useful if you want to keep their messages for other games)
    "short_name": 1,
  }
}
```

3. Install `imclassif.py` dependencies by running
```
pip3 install --user -r requirements.txt
```
