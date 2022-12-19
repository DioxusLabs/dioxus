# Práticas Recomendadas

## Componentes Reutilizáveis

Tanto quanto possível, divida seu código em pequenos componentes e _hooks_ reutilizáveis, em vez de implementar grandes partes da interface do usuário em um único componente. Isso ajudará você a manter o código sustentável – é muito mais fácil, por exemplo, adicionar, remover ou reordenar partes da interface do usuário se ela estiver organizada em componentes.

Organize seus componentes em módulos para manter a base de código fácil de navegar!

## Minimize as Dependências do Estado

Embora seja possível compartilhar o estado entre os componentes, isso só deve ser feito quando necessário. Qualquer componente associado a um objeto de estado específico precisa ser renderizado novamente quando esse estado for alterado. Por esta razão:

- Mantenha o estado local para um componente, se possível
- Ao compartilhar o estado por meio de adereços, passe apenas os dados específicos necessários
