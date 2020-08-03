# Джобот

_Taki_ ([Polish](https://en.wiktionary.org/wiki/taki#Polish) for _such_) is a guessing game
for chatrooms where people think they know each other well :^)

## Acknowledgements

_WDYT_ and _Joker_ functionality is ported from [sunDalik/vk-bot](https://github.com/sunDalik/vk-bot).

## Legacy versions

* [Telegram](https://github.com/texbois/joebot/tree/telegram/) (pretty lightweight thanks to a handwritten API client)

## Getting up & running

1. Use [VkOpt](https://chrome.google.com/webstore/detail/vkopt/hoboppgpbgclpfnjfdidokiilachfcbb)
to export any VKontakte chat you'd like to use for the _Taki_ game
(be sure to select the _Export as .html_ option)
2. Save the chat history file as `messages.html` and place it in the crate root
3. Build the bot by running `cargo build --release`
4. Specify participants in the `MSG_NAMES` environment variable and run the bot

Example command:
```
MSG_NAMES="Ivan Ivanoff, Pavel Pavlov, Alexey Alexeev" cargo run --release
```

## Image recognition

```
pip3 install --user -r requirements.txt
```
