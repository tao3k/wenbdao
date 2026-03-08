# Attachment and Embed Fixture

Inline embedded wikilink should be ignored:
prefix ![[graph-b]]

Punctuation before a normal wikilink must still parse:
Warning! [[graph-c]]

Attachment links must be ignored:
![Image](assets/pic.png)
[PDF](files/manual.pdf)
[Absolute Attachment](/tmp/manual.pdf)
[Attachment URI](file:///tmp/manual.pdf)
