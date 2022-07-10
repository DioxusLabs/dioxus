# Efeitos

Em teoria, seu código de interface do usuário deve ser livre de efeitos colaterais. Sempre que um componente é renderizado, todo o seu estado deve ser preparado com antecedência. Na realidade, muitas vezes precisamos realizar algum tipo de efeito colateral. Os efeitos possíveis incluem:

- Registrando alguns dados
- Pré-buscando alguns dados
- Anexar código a elementos nativos
- Limpando

Esta seção está organizada em interatividade porque os efeitos podem ser importantes para adicionar coisas como transições, vídeos e outras mídias importantes.
