# Subárvores

Uma maneira de estender a `VirtualDom` do Dioxus é através do uso de "Subárvores".

As subárvores são pedaços da árvore do `VirtualDom` distintas do resto da árvore.

Eles ainda participam da propagação de eventos, `diffing`, etc, mas terão um conjunto separado de edições geradas durante a fase de `diff`.

Para um `VirtualDom` que possui uma árvore raiz com duas subárvores, as edições seguem um padrão de:

Raiz
-> Árvore 1
-> Árvore 2
-> Raiz original da árvore

- Edição da Raiz
- Edição da Ávore 1
- Edição da Ávore 2
- Edição da Raiz

O objetivo dessa funcionalidade é habilitar coisas como Portais, Windows e renderizadores alternativos embutidos sem a necessidade de criar um novo `VirtualDom`.

Com os plugins de renderização corretos, uma subárvore pode ser renderizada como qualquer coisa - uma cena 3D, SVG ou até mesmo como o conteúdo de uma nova janela ou modal.

Essa funcionalidade é semelhante aos "Portais" no React, mas muito mais "agnóstico do renderizador".

Os portais, por natureza, não são necessariamente multiplataforma e dependem da funcionalidade do renderizador, portanto, faz sentido abstrair seu propósito no conceito de subárvore.

O renderizador de desktop vem pré-carregado com os plugins de subárvore de janela e notificação, tornando possível renderizar subárvores em janelas totalmente diferentes.

As subárvores também resolvem os problemas de "ponte" no React, onde dois renderizadores diferentes precisam de dois `VirtualDoms` diferentes para funcionar corretamente.

No Dioxus, você só precisa de um `VirtualDom` e dos plugins de renderização corretos.

## API

Devido à sua importância na hierarquia, Componentes - não nós - são tratados como raízes de subárvores.

```rust

fn Subtree<P>(cx: Scope<P>) -> DomTree {

}

fn Window() -> DomTree {
    Subtree {
        onassign: move |e| {
            // create window
        }
        children()
    }
}

fn 3dRenderer -> DomTree {
    Subtree {
        onassign: move |e| {
            // initialize bevy
        }
    }
}

```
