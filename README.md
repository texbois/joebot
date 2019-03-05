# Джобот

_Taki_ ([Polish](https://en.wiktionary.org/wiki/taki#Polish) for _such_) is a guessing game
for chatrooms where people think they know each other well :^)

## Getting up & running

1. Use [VkOpt](https://chrome.google.com/webstore/detail/vkopt/hoboppgpbgclpfnjfdidokiilachfcbb)
to export any VKontakte chat you'd like to use for the _Taki_ game
(be sure to select the _Export as .html_ option)
2. Save the chat history file as `messages.html` and place it in the crate root
3. Build the bot by running `cargo build --release`

You can ignore messages from a particular user (e.g. a bot) by putting their screen name
in the `TAKI_IGNORE_NAMES` environment variable when running the build. You can enter
multiple names, too, just separate them with commas. Example command:
```
TAKI_IGNORE_NAMES="id1,id2,custom_name" cargo build --release
```

## Why Telegram?

Joe was initially developed as a VK chat bot. However, the site's administration
[forbids sending messages from the server](https://vk.com/faq13567).
