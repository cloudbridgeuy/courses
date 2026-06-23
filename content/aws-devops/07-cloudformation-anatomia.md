+++
title = "Anatomía de un template — CloudFormation"
+++

:::title-slide Semana 2
:::

## De la caja negra al código

La Semana 1 terminó con la aplicación en línea, desplegada a partir de un archivo que
tratamos como una caja negra: `taller-semana1.yaml`. Esta semana abrimos esa caja.

El objetivo no es memorizar la sintaxis de CloudFormation, sino **saber leer un
template**: reconocer qué describe, encontrar dónde se define cada recurso, y entender
cómo se conectan entre sí. Esa lectura es lo que convierte la infraestructura de algo
que "alguien configuró una vez" en algo que el equipo entiende, revisa, y modifica.

## Por qué infraestructura como código

Configurar recursos a mano desde la consola —*click-ops*— funciona una vez. El
problema aparece después: nadie recuerda exactamente qué se configuró, no hay registro
de los cambios, y reproducir el mismo ambiente en otra región o cuenta significa
repetir decenas de clics sin garantía de que el resultado sea idéntico.

La **infraestructura como código** (IaC) resuelve esto describiendo los recursos en un
archivo de texto versionado. El archivo es la fuente de verdad: se revisa en un *pull
request*, se guarda en el repositorio junto al código, y produce siempre el mismo
ambiente. La diferencia es la misma que vimos entre subir archivos a mano y hacer
`git push`: un proceso reproducible en lugar de una serie de pasos manuales.

:::inline-slide light
## Click-ops vs. infraestructura como código

| Click-ops | Infraestructura como código |
| --- | --- |
| Pasos manuales en la consola | Recursos descritos en un archivo |
| Sin registro de cambios | Versionado en git |
| Difícil de reproducir | Idéntico en cada lanzamiento |
| El conocimiento vive en una persona | El conocimiento vive en el repositorio |
:::

## Las secciones de un template

Un template de CloudFormation es un archivo YAML (o JSON) con un conjunto de secciones
de nivel superior. Solo una es obligatoria —`Resources`—; las demás son opcionales y
aparecen según lo que el template necesite.

| Sección | Para qué sirve |
| --- | --- |
| `Resources` | **Obligatoria.** Los recursos a crear: la tabla, el clúster, el balanceador. |
| `Parameters` | Valores que se proveen al lanzar el stack (por ejemplo, el URI de la imagen). |
| `Outputs` | Valores que el stack expone al terminar (por ejemplo, la URL del ALB). |
| `Mappings` | Tablas de búsqueda fijas (por ejemplo, una AMI distinta por región). |
| `Conditions` | Reglas que activan o desactivan recursos según los parámetros. |

En la Semana 1 ya interactuó con tres de ellas sin saberlo: completó un **parámetro**
(el URI de la imagen), y leyó un **output** (la URL del ALB) que el template expuso al
llegar a `CREATE_COMPLETE`.

## Recursos: nombre lógico y nombre físico

Cada recurso dentro de `Resources` tiene un **nombre lógico** (*logical ID*): el
identificador que usted le da dentro del template. CloudFormation, al crear el recurso,
le asigna además un **nombre físico**: el identificador real en AWS.

```yaml
Resources:
  TablaApp:                          # nombre lógico (lo elige usted)
    Type: AWS::DynamoDB::Table
    Properties:
      TableName: taller-datos        # nombre físico (opcional)
      BillingMode: PAY_PER_REQUEST
```

El nombre lógico (`TablaApp`) es cómo el resto del template se refiere a este recurso.
El nombre físico (`taller-datos`) es cómo aparece en la consola de DynamoDB. Si no
especifica un nombre físico, CloudFormation genera uno único automáticamente —una
práctica habitual, porque evita colisiones de nombres al lanzar el mismo template
varias veces.

## Funciones intrínsecas: conectar los recursos

Los recursos rara vez son independientes: el servicio ECS necesita el nombre del
clúster, el ALB necesita el ID de la subred, la *task definition* necesita el URI de
la imagen. Las **funciones intrínsecas** permiten referirse a un valor que solo se
conoce cuando el stack se crea, sin escribirlo a mano.

Las tres más comunes:

- **`!Ref`** — devuelve el valor de un parámetro, o el nombre físico de un recurso.

  ```yaml
  Image: !Ref ImageUri        # el valor del parámetro ImageUri
  ```

- **`!GetAtt`** — devuelve un atributo específico de un recurso.

  ```yaml
  DNSName: !GetAtt BalanceadorApp.DNSName   # el DNS del ALB creado
  ```

- **`!Sub`** — sustituye variables dentro de una cadena de texto.

  ```yaml
  Mensaje: !Sub "Tabla ${TablaApp} en la región ${AWS::Region}"
  ```

`!Ref` y `!GetAtt` son la forma corta de `Fn::Ref` y `Fn::GetAtt`; ambas notaciones
son equivalentes y se encontrará con las dos al leer templates de la documentación de
AWS.

::: extra ¿Por qué YAML y no JSON?
CloudFormation acepta los dos formatos, y son intercambiables. Este taller usa **YAML**
porque admite comentarios (líneas con `#`), es más compacto, y ofrece la sintaxis corta
de las funciones intrínsecas (`!Ref` en lugar de `{ "Ref": "..." }`). JSON sigue siendo
común en templates generados por herramientas. Saber leer ambos es útil; para escribir
a mano, YAML es más cómodo.
:::

:::slide
## Las funciones intrínsecas

- **`!Ref`** — el valor de un parámetro, o el nombre de un recurso.
- **`!GetAtt`** — un atributo de un recurso (`Recurso.Atributo`).
- **`!Sub`** — sustituir variables dentro de un texto.

Conectan recursos sin escribir valores a mano.
:::
