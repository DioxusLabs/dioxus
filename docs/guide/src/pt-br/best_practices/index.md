# Práticas Recomendadas

## Componentes Reutilizáveis

Tanto quanto possível, divida seu código em pequenos componentes e _hooks_ reutilizáveis, em vez de implementar grandes partes da interface do usuário em um único componente. Isso ajudará você a manter o código sustentável – é muito mais fácil, por exemplo, adicionar, remover ou reordenar partes da interface do usuário se ela estiver organizada em componentes.

Organize seus componentes em módulos para manter a base de código fácil de navegar!

## Minimize as Dependências do Estado

Embora seja possível compartilhar o estado entre os componentes, isso só deve ser feito quando necessário. Qualquer componente associado a um objeto de estado específico precisa ser renderizado novamente quando esse estado for alterado. Por esta razão:

- Mantenha o estado local para um componente, se possível
- Ao compartilhar o estado por meio de adereços, passe apenas os dados específicos necessários

## Bibliotecas Reutilizáveis

Ao publicar uma biblioteca projetada para funcionar com o Dioxus, é altamente recomendável usar apenas o recurso principal na `crate` `dioxus`. Isso faz com que sua `crate` seja compilada mais rapidamente, mais estável e evita a inclusão de bibliotecas incompatíveis que podem fazer com que ela não seja compilada em plataformas não suportadas.

❌ Não inclua dependências desnecessárias nas bibliotecas:

```toml
dioxus = { version = "...", features = ["web", "desktop", "full"]}
```

✅ Adicione apenas os recursos que você precisa:

```toml
dioxus = { version = "...", features = "core"}
```
