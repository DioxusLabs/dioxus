# Walkthrough of the Hello World Example Internals

This walkthrough will take you through the internals of the Hello World example program. It will explain how major parts of Dioxus interals interact with each other to go from a source file to a running program.

## The Source File

We start will a hello world program. This program renders a desktop app with the text "Hello World" in a webview.

```rust
{{#include ../../../../../examples/readme.rs}}
```

[![](https://mermaid.ink/img/pako:eNqNkT1vwyAQhv8KvSlR48HphtQtqjK0S6tuSBGBS0CxwcJHk8rxfy_YVqxKVdR3ug_u4YXrQHmNwOFQ-bMyMhB7fReOJbVxfwyyMSy0l7GSpW1ARda727ksUy5MuSyKgvBC5ULA1h5N8WK_kCkfHWHgrBuiXsBynrvdsY9E3u1iM_eyvFOVVadMnELOap-o1911JLPHZ1b-YqLTc3LjTt7WifTZMJPsPdx1ov3Z_ellfcdL8R8vmTy5eUqsTUpZ-vzZzjAEK6gx1NLqtJwuNwSQwRoF8BRqGU4ChOvTORnJf3w7BZxCxBXERkvCjZXpQTXwg6zaVEVtyYe3cdvD0vsf4bucgw?type=png)](https://mermaid.live/edit#pako:eNqNkT1vwyAQhv8KvSlR48HphtQtqjK0S6tuSBGBS0CxwcJHk8rxfy_YVqxKVdR3ug_u4YXrQHmNwOFQ-bMyMhB7fReOJbVxfwyyMSy0l7GSpW1ARda727ksUy5MuSyKgvBC5ULA1h5N8WK_kCkfHWHgrBuiXsBynrvdsY9E3u1iM_eyvFOVVadMnELOap-o1911JLPHZ1b-YqLTc3LjTt7WifTZMJPsPdx1ov3Z_ellfcdL8R8vmTy5eUqsTUpZ-vzZzjAEK6gx1NLqtJwuNwSQwRoF8BRqGU4ChOvTORnJf3w7BZxCxBXERkvCjZXpQTXwg6zaVEVtyYe3cdvD0vsf4bucgw)

## The rsx! Macro

Before rust runs the program, it will expand all macros. Here is what the hello world example looks like expanded:

```rust
{{#include ../../../examples/readme_expanded.rs}}
```

The rsx macro separates the static parts of the rsx (the template) and the dynamic parts (the dynamic_nodes and dynamic_attributes).

The static template only contains the parts of the rsx that cannot change at runtime with holes for the dynamic parts:

[![](https://mermaid.ink/img/pako:eNqdksFuwjAMhl8l8wkkKtFx65njdtm0E0GVSQKJoEmVOgKEeHecUrXStO0wn5Lf9u8vcm6ggjZQwf4UzspiJPH2Ib3g6NLuELG1oiMkp0TsLs9EDu2iUeSCH8tz2HJmy3lRFPrqsXGq9mxeLzcbCU6LZSUGXWRdwnY7tY7Tdoko-Dq1U64fODgiUfzJMeuOe7_ZGq-ny2jNhGQu9DqT8NUK6w72RcL8dxgdzv4PnHLAKf-Fk80HoBUDrfkqeBkTUd8EC2hMbNBpXtYtJySQNQ0PqPioMR4lSH_nOkwUPq9eQUUxmQWkViOZtUN-UwPVHk8dq0Y7CvH9uf3-E9wfrmuk1A?type=png)](https://mermaid.live/edit#pako:eNqdksFuwjAMhl8l8wkkKtFx65njdtm0E0GVSQKJoEmVOgKEeHecUrXStO0wn5Lf9u8vcm6ggjZQwf4UzspiJPH2Ib3g6NLuELG1oiMkp0TsLs9EDu2iUeSCH8tz2HJmy3lRFPrqsXGq9mxeLzcbCU6LZSUGXWRdwnY7tY7Tdoko-Dq1U64fODgiUfzJMeuOe7_ZGq-ny2jNhGQu9DqT8NUK6w72RcL8dxgdzv4PnHLAKf-Fk80HoBUDrfkqeBkTUd8EC2hMbNBpXtYtJySQNQ0PqPioMR4lSH_nOkwUPq9eQUUxmQWkViOZtUN-UwPVHk8dq0Y7CvH9uf3-E9wfrmuk1A)

The dynamic_nodes and dynamic_attributes are the parts of the rsx that can change at runtime:

[![](https://mermaid.ink/img/pako:eNp1UcFOwzAM_RXLVzZpvUbighDiABfgtkxTlnirtSaZUgc0df130hZEEcwny35-79nu0EZHqHDfxA9bmyTw9KIDlGjz7pDMqQZ3DsazhVCQ7dQbwnEiKxwDvN3NqhN4O4C3q_VaIztYKXjkQ7184HcCG3MQSgq6Mes1bjbTPAV3RdqIJN5l-V__2_Fcf5iY68dgG7ZHBT4WD5ftZfIBN7dQ_Tj4w1B9MVTXGZa_GMYdcIGekjfsymW7oaFRavKkUZXUmXTUqENfcCZLfD0Hi0pSpgXmkzNC92zKATyqvWnaUiXHEtPz9KrxY_0nzYOPmA?type=png)](https://mermaid.live/edit#pako:eNp1UcFOwzAM_RXLVzZpvUbighDiABfgtkxTlnirtSaZUgc0df130hZEEcwny35-79nu0EZHqHDfxA9bmyTw9KIDlGjz7pDMqQZ3DsazhVCQ7dQbwnEiKxwDvN3NqhN4O4C3q_VaIztYKXjkQ7184HcCG3MQSgq6Mes1bjbTPAV3RdqIJN5l-V__2_Fcf5iY68dgG7ZHBT4WD5ftZfIBN7dQ_Tj4w1B9MVTXGZa_GMYdcIGekjfsymW7oaFRavKkUZXUmXTUqENfcCZLfD0Hi0pSpgXmkzNC92zKATyqvWnaUiXHEtPz9KrxY_0nzYOPmA)
