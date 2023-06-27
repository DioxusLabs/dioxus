# Walkthrough of the Hello World Example Internals

This walkthrough will take you through the internals of the Hello World example program. It will explain how major parts of Dioxus internals interact with each other to take the readme example from a source file to a running application. This guide should serve as a high-level overview of the internals of Dioxus. It is not meant to be a comprehensive guide.

## The Source File

We start will a hello world program. This program renders a desktop app with the text "Hello World" in a webview.

```rust, no_run
{{#include ../../../../../examples/readme.rs}}
```

[![](https://mermaid.ink/img/pako:eNqNkT1vwyAQhv8KvSlR48HphtQtqjK0S6tuSBGBS0CxwcJHk8rxfy_YVqxKVdR3ug_u4YXrQHmNwOFQ-bMyMhB7fReOJbVxfwyyMSy0l7GSpW1ARda727ksUy5MuSyKgvBC5ULA1h5N8WK_kCkfHWHgrBuiXsBynrvdsY9E3u1iM_eyvFOVVadMnELOap-o1911JLPHZ1b-YqLTc3LjTt7WifTZMJPsPdx1ov3Z_ellfcdL8R8vmTy5eUqsTUpZ-vzZzjAEK6gx1NLqtJwuNwSQwRoF8BRqGU4ChOvTORnJf3w7BZxCxBXERkvCjZXpQTXwg6zaVEVtyYe3cdvD0vsf4bucgw?type=png)](https://mermaid.live/edit#pako:eNqNkT1vwyAQhv8KvSlR48HphtQtqjK0S6tuSBGBS0CxwcJHk8rxfy_YVqxKVdR3ug_u4YXrQHmNwOFQ-bMyMhB7fReOJbVxfwyyMSy0l7GSpW1ARda727ksUy5MuSyKgvBC5ULA1h5N8WK_kCkfHWHgrBuiXsBynrvdsY9E3u1iM_eyvFOVVadMnELOap-o1911JLPHZ1b-YqLTc3LjTt7WifTZMJPsPdx1ov3Z_ellfcdL8R8vmTy5eUqsTUpZ-vzZzjAEK6gx1NLqtJwuNwSQwRoF8BRqGU4ChOvTORnJf3w7BZxCxBXERkvCjZXpQTXwg6zaVEVtyYe3cdvD0vsf4bucgw)

## The rsx! Macro

Before the Rust compiler runs the program, it will expand all macros. Here is what the hello world example looks like expanded:

```rust, no_run
{{#include ../../../examples/readme_expanded.rs}}
```

The rsx macro separates the static parts of the rsx (the template) and the dynamic parts (the dynamic_nodes and dynamic_attributes).

The static template only contains the parts of the rsx that cannot change at runtime with holes for the dynamic parts:

[![](https://mermaid.ink/img/pako:eNqdksFuwjAMhl8l8wkkKtFx65njdtm0E0GVSQKJoEmVOgKEeHecUrXStO0wn5Lf9u8vcm6ggjZQwf4UzspiJPH2Ib3g6NLuELG1oiMkp0TsLs9EDu2iUeSCH8tz2HJmy3lRFPrqsXGq9mxeLzcbCU6LZSUGXWRdwnY7tY7Tdoko-Dq1U64fODgiUfzJMeuOe7_ZGq-ny2jNhGQu9DqT8NUK6w72RcL8dxgdzv4PnHLAKf-Fk80HoBUDrfkqeBkTUd8EC2hMbNBpXtYtJySQNQ0PqPioMR4lSH_nOkwUPq9eQUUxmQWkViOZtUN-UwPVHk8dq0Y7CvH9uf3-E9wfrmuk1A?type=png)](https://mermaid.live/edit#pako:eNqdksFuwjAMhl8l8wkkKtFx65njdtm0E0GVSQKJoEmVOgKEeHecUrXStO0wn5Lf9u8vcm6ggjZQwf4UzspiJPH2Ib3g6NLuELG1oiMkp0TsLs9EDu2iUeSCH8tz2HJmy3lRFPrqsXGq9mxeLzcbCU6LZSUGXWRdwnY7tY7Tdoko-Dq1U64fODgiUfzJMeuOe7_ZGq-ny2jNhGQu9DqT8NUK6w72RcL8dxgdzv4PnHLAKf-Fk80HoBUDrfkqeBkTUd8EC2hMbNBpXtYtJySQNQ0PqPioMR4lSH_nOkwUPq9eQUUxmQWkViOZtUN-UwPVHk8dq0Y7CvH9uf3-E9wfrmuk1A)

The dynamic_nodes and dynamic_attributes are the parts of the rsx that can change at runtime:

[![](https://mermaid.ink/img/pako:eNp1UcFOwzAM_RXLVzZpvUbighDiABfgtkxTlnirtSaZUgc0df130hZEEcwny35-79nu0EZHqHDfxA9bmyTw9KIDlGjz7pDMqQZ3DsazhVCQ7dQbwnEiKxwDvN3NqhN4O4C3q_VaIztYKXjkQ7184HcCG3MQSgq6Mes1bjbTPAV3RdqIJN5l-V__2_Fcf5iY68dgG7ZHBT4WD5ftZfIBN7dQ_Tj4w1B9MVTXGZa_GMYdcIGekjfsymW7oaFRavKkUZXUmXTUqENfcCZLfD0Hi0pSpgXmkzNC92zKATyqvWnaUiXHEtPz9KrxY_0nzYOPmA?type=png)](https://mermaid.live/edit#pako:eNp1UcFOwzAM_RXLVzZpvUbighDiABfgtkxTlnirtSaZUgc0df130hZEEcwny35-79nu0EZHqHDfxA9bmyTw9KIDlGjz7pDMqQZ3DsazhVCQ7dQbwnEiKxwDvN3NqhN4O4C3q_VaIztYKXjkQ7184HcCG3MQSgq6Mes1bjbTPAV3RdqIJN5l-V__2_Fcf5iY68dgG7ZHBT4WD5ftZfIBN7dQ_Tj4w1B9MVTXGZa_GMYdcIGekjfsymW7oaFRavKkUZXUmXTUqENfcCZLfD0Hi0pSpgXmkzNC92zKATyqvWnaUiXHEtPz9KrxY_0nzYOPmA)

## Launching the App

The app is launched by calling the `launch` function with the root component. Internally, this function will create a new web view using [wry](https://docs.rs/wry/latest/wry/) and create a virtual dom with the root component. This guide will not explain the renderer in-depth, but you can read more about it in the [custom renderer](/guide/custom-renderer) section.

## The Virtual DOM

Before we dive into the initial render in the virtual dom, we need to discuss what the virtual dom is. The virtual dom is a representation of the dom that is used to diff the current dom from the new dom. This diff is then used to create a list of mutations that need to be applied to the dom.

The Virtual Dom roughly looks like this:

```rust, no_run
pub struct VirtualDom {
    // All the templates that have been created or set durring hot reloading
    pub(crate) templates: FxHashMap<TemplateId, FxHashMap<usize, Template<'static>>>,

    // A slab of all the scopes that have been created
    pub(crate) scopes: ScopeSlab,

    // All scopes that have been marked as dirty
    pub(crate) dirty_scopes: BTreeSet<DirtyScope>,

    // Every element is actually a dual reference - one to the template and the other to the dynamic node in that template
    pub(crate) elements: Slab<ElementRef>,

    // This receiver is used to receive messages from hooks about what scopes need to be marked as dirty
    pub(crate) rx: futures_channel::mpsc::UnboundedReceiver<SchedulerMsg>,

    // The changes queued up to be sent to the renderer
    pub(crate) mutations: Mutations<'static>,
}
```

> What is a [slab](https://docs.rs/slab/latest/slab/)?
> A slab acts like a hashmap with integer keys if you don't care about the value of the keys. It is internally backed by a dense vector which makes it more efficient than a hashmap. When you insert a value into a slab, it returns an integer key that you can use to retrieve the value later.

> How does Dioxus use slabs?
> Dioxus uses "synchronized slabs" to communicate between the renderer and the VDOM. When an node is created in the Virtual Dom, a ElementId is passed along with the mutation to the renderer to identify the node. These ids are used by the Virtual Dom to reference that nodes in future mutations like setting an attribute on a node or removing a node.
> When the renderer sends an event to the Virtual Dom, it sends the ElementId of the node that the event was triggered on. The Virtual Dom uses this id to find the node in the slab and then run the necessary event handlers.

The virtual dom is a tree of scopes. A new scope is created for every component when it is first rendered and recycled when the component is unmounted.

Scopes serve three main purposes:

1. They store the state of hooks used by the component
2. They store the state for the context API
3. They store the current and previous VNode that was rendered for diffing

### The Initial Render

The root scope is created and rebuilt:

1. The root component is run
2. The root component returns a VNode
3. Mutations for the VNode are created and added to the mutation list (this may involve creating new child components)
4. The VNode is stored in the root scope

After the root scope is built, the mutations are sent to the renderer to be applied to the dom.

After the initial render, the root scope looks like this:

[![](https://mermaid.ink/img/pako:eNqtVE1P4zAQ_SuzPrWikRpWXCLtBRDisItWsOxhCaqM7RKricdyJrQV8N93QtvQNCkfEnOynydv3nxkHoVCbUQipjnOVSYDwc_L1AFbWd3dB-kzuEQkuFLoDUwDFkCZAek9nGDh0RlHK__atA1GkUUHf45f0YbppAqB_aOzIAvz-t7-chN_Y-1bw1WSJKsglIu2w9tktWXxIIuHURT5XCqTYa5NmDguw2R8c5MKq2GcgF46WTB_jafi9rZL0yi5q4jQTSrf9altO4okCn1Ratwyz55Qxuku2ITlTMgs6HCQimsPmb3PvqVi-L5gjXP3QcnxWnL8JZLrwGvR31n0KV-Bx6-r-oVkT_-3G1S-NQLbk9i8rj7udP2cixed2QcDCitHJiQw7ub3EVlNecrPjudG2-6soFO5VbMECmR9T5OnlUY4-AFxfw9aTFst3McU9TK1Otm6NEn_DubBYlX2_dglLXOz48FgwJmJ5lZTlhz6xWgNaFnyDgpymcARHO0W2a9J_l5w2wYXvHuGPcqaQ-rESBQmFNJq3nCPNZoK3l4sUSR81DLMUpG6Z_aTFeHV0imRUKjMSFReSzKnVnKGhUimMi8ZNdoShl-rlfmyOUfCS_cPcePz_B_Wl4pc?type=png)](https://mermaid.live/edit#pako:eNqtVE1P4zAQ_SuzPrWikRpWXCLtBRDisItWsOxhCaqM7RKricdyJrQV8N93QtvQNCkfEnOynydv3nxkHoVCbUQipjnOVSYDwc_L1AFbWd3dB-kzuEQkuFLoDUwDFkCZAek9nGDh0RlHK__atA1GkUUHf45f0YbppAqB_aOzIAvz-t7-chN_Y-1bw1WSJKsglIu2w9tktWXxIIuHURT5XCqTYa5NmDguw2R8c5MKq2GcgF46WTB_jafi9rZL0yi5q4jQTSrf9altO4okCn1Ratwyz55Qxuku2ITlTMgs6HCQimsPmb3PvqVi-L5gjXP3QcnxWnL8JZLrwGvR31n0KV-Bx6-r-oVkT_-3G1S-NQLbk9i8rj7udP2cixed2QcDCitHJiQw7ub3EVlNecrPjudG2-6soFO5VbMECmR9T5OnlUY4-AFxfw9aTFst3McU9TK1Otm6NEn_DubBYlX2_dglLXOz48FgwJmJ5lZTlhz6xWgNaFnyDgpymcARHO0W2a9J_l5w2wYXvHuGPcqaQ-rESBQmFNJq3nCPNZoK3l4sUSR81DLMUpG6Z_aTFeHV0imRUKjMSFReSzKnVnKGhUimMi8ZNdoShl-rlfmyOUfCS_cPcePz_B_Wl4pc)

### Waiting for Events

The Virtual Dom will only ever rerender a scope if it is marked as dirty. Each hook is responsible for marking the scope as dirty if the state has changed. Hooks can mark a scope as dirty by sending a message to the Virtual Dom's channel.

There are generally two ways a scope is marked as dirty:

1. The renderer triggers an event: This causes an event listener to be called if needed which may mark a component as dirty
2. The renderer calls wait for work: This polls futures which may mark a component as dirty

Once at least one scope is marked as dirty, the renderer can call `render_with_deadline` to diff the dirty scopes.

### Diffing Scopes

If the user clicked the "up high" button, the root scope would be marked as dirty by the use_state hook. Once the desktop renderer calls `render_with_deadline`, the root scope would be diffed.

To start the diffing process, the component is run. After the root component is run it will look like this:

[![](https://mermaid.ink/img/pako:eNrFVlFP2zAQ_iuen0BrpCaIl0i8AEJ72KQJtpcRFBnbJVYTn-U4tBXw33dpG5M2CetoBfdkny_ffb67fPIT5SAkjekkhxnPmHXk-3WiCVpZ3T9YZjJyDeDIDQcjycRCQVwmCTOGXEBhQEvtVvG1CWUldwo0-XX-6vVIF5W1GB9cWVbI1_PNL5v8jW3uPFbpmFOc2HK-GfA2WG1ZeJSFx0EQmJxxmUEupE01liEd394mVAkyjolYaFYgfu1P6N1dF8Yzua-cA51WphtTWzsLc872Zan9CnEGUkktuk6fFm_i5NxFRwn9bUimHrIvCT3-N2EBM70j5XBNOTwI5TrxmvQJkr7ELcHx67Jeggz0v92g8q0RaE-iP1193On6NyxecKUeJeFQaSdtTMLu_Xah5ctT_u94Nty2ZwU0zxWfxqQA5PecPq84kq9nfRw7SK0WDiEFZ4O37d34S_-08lFBVfb92KVb5HIrAp0WpjKYKeGyODLz0dohWIkaZNkiJqfkdLvIH6oRaTSoEmm0n06k0a5K0ZdpL61Io0Yt0nfpxc7UQ0_9cJrhyZ8syX-6brS706Mc489Vjja7fbWj3cxDqIdfJJqOaCFtwZTAV8hT7U0ovjBQRmiMS8HsNKGJfsE4Vjm4WWhOY2crOaKVEczJS8WwgAWNJywv0SuFcmB_rJ41y9fNiBqm_wA0MS9_AUuAiy0?type=png)](https://mermaid.live/edit#pako:eNrFVlFP2zAQ_iuen0BrpCaIl0i8AEJ72KQJtpcRFBnbJVYTn-U4tBXw33dpG5M2CetoBfdkny_ffb67fPIT5SAkjekkhxnPmHXk-3WiCVpZ3T9YZjJyDeDIDQcjycRCQVwmCTOGXEBhQEvtVvG1CWUldwo0-XX-6vVIF5W1GB9cWVbI1_PNL5v8jW3uPFbpmFOc2HK-GfA2WG1ZeJSFx0EQmJxxmUEupE01liEd394mVAkyjolYaFYgfu1P6N1dF8Yzua-cA51WphtTWzsLc872Zan9CnEGUkktuk6fFm_i5NxFRwn9bUimHrIvCT3-N2EBM70j5XBNOTwI5TrxmvQJkr7ELcHx67Jeggz0v92g8q0RaE-iP1193On6NyxecKUeJeFQaSdtTMLu_Xah5ctT_u94Nty2ZwU0zxWfxqQA5PecPq84kq9nfRw7SK0WDiEFZ4O37d34S_-08lFBVfb92KVb5HIrAp0WpjKYKeGyODLz0dohWIkaZNkiJqfkdLvIH6oRaTSoEmm0n06k0a5K0ZdpL61Io0Yt0nfpxc7UQ0_9cJrhyZ8syX-6brS706Mc489Vjja7fbWj3cxDqIdfJJqOaCFtwZTAV8hT7U0ovjBQRmiMS8HsNKGJfsE4Vjm4WWhOY2crOaKVEczJS8WwgAWNJywv0SuFcmB_rJ41y9fNiBqm_wA0MS9_AUuAiy0)

Next, the Virtual Dom will compare the new VNode with the previous VNode and only update the parts of the tree that have changed.

When a component is re-rendered, the Virtual Dom will compare the new VNode with the previous VNode and only update the parts of the tree that have changed.

The diffing algorithm goes through the list of dynamic attributes and nodes and compares them to the previous VNode. If the attribute or node has changed, a mutation that describes the change is added to the mutation list.

Here is what the diffing algorithm looks like for the root scope (red lines indicate that a mutation was generated, and green lines indicate that no mutation was generated)

[![](https://mermaid.ink/img/pako:eNrFlFFPwjAQx7_KpT7Kko2Elya8qCE-aGLAJ5khpe1Yw9Zbug4k4He3OJjbGPig0T5t17tf_nf777aEo5CEkijBNY-ZsfAwDjW4kxfzhWFZDGNECxOOmYTIYAo2lsCyDG4xzVBLbcv8_RHKSG4V6orSIN0Wxrh8b2RYKr_uTyubd1W92GiWKg7aac6bOU3G803HbVk82xfP_Ok0JEqAT-FeLWJvpFYSOBbaSkMhCMnra5MgtfhWFrPWqHlhL2urT6atbU-oa0PNE8WXFFJ0-nazXakRroddGk9IwYEUnCd5w7Pddr5UTT8ZuVJY5F0fM7ebRLYyXNDgUnprJWxM-9lb7xAQLHe-M2xDYQCD9pD_2hez_kVn-P_rjLq6n3qjYv2iO5qz9DyvPdyv1ETp5eTTJ_7BGvQq8v1TVtl5jXUcRRcrqFh-dI4VtFlBN6t_ynLNkh5JpUmZEm5rbvfhkLiN6H4BQt2jYGYZklC_uzxWWJxsNCfUmkL2SJEJZuWdYs4cKaERS3IXlUJZNI_lGv7cxj2SMf2CeMx5_wBcbK19?type=png)](https://mermaid.live/edit#pako:eNrFlFFPwjAQx7_KpT7Kko2Elya8qCE-aGLAJ5khpe1Yw9Zbug4k4He3OJjbGPig0T5t17tf_nf777aEo5CEkijBNY-ZsfAwDjW4kxfzhWFZDGNECxOOmYTIYAo2lsCyDG4xzVBLbcv8_RHKSG4V6orSIN0Wxrh8b2RYKr_uTyubd1W92GiWKg7aac6bOU3G803HbVk82xfP_Ok0JEqAT-FeLWJvpFYSOBbaSkMhCMnra5MgtfhWFrPWqHlhL2urT6atbU-oa0PNE8WXFFJ0-nazXakRroddGk9IwYEUnCd5w7Pddr5UTT8ZuVJY5F0fM7ebRLYyXNDgUnprJWxM-9lb7xAQLHe-M2xDYQCD9pD_2hez_kVn-P_rjLq6n3qjYv2iO5qz9DyvPdyv1ETp5eTTJ_7BGvQq8v1TVtl5jXUcRRcrqFh-dI4VtFlBN6t_ynLNkh5JpUmZEm5rbvfhkLiN6H4BQt2jYGYZklC_uzxWWJxsNCfUmkL2SJEJZuWdYs4cKaERS3IXlUJZNI_lGv7cxj2SMf2CeMx5_wBcbK19)

## Conclusion

This is only a brief overview of how the Virtual Dom works. There are several aspects not yet covered in this guide including how the Virtual Dom handles async-components, keyed diffing, and how it uses [bump allocation](https://github.com/fitzgen/bumpalo) to efficiently allocate VNodes. If need more information about the Virtual Dom, you can read the code of the [core](https://github.com/DioxusLabs/dioxus/tree/master/packages/core) crate or reach out to us on [Discord](https://discord.gg/XgGxMSkvUM).
