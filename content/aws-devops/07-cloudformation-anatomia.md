+++
title = "Anatomía de un template"
+++

:::title-slide Semana 2 - CloudFormation
:::

## Interpretando CloudFormation

La Semana 1 terminó con la aplicación en línea, desplegada a partir de un archivo que
tratamos como una caja negra: `taller-aws-devops-semana1.yaml`. Esta semana se ve
cómo se compone un archivo de CloudFormation.

El objetivo no es memorizar la sintaxis de CloudFormation, sino **saber leer un
template**: reconocer qué describe, encontrar dónde se define cada recurso, y entender
cómo se conectan entre sí. Esa lectura es lo que convierte la infraestructura de algo
que "alguien configuró una vez" en algo que el equipo entiende, revisa, y modifica.

## Por qué infraestructura como código

Configurar recursos a mano desde la consola (*click-ops*) funciona una vez. El
problema aparece después: nadie recuerda exactamente qué se configuró, no hay registro
de los cambios, y reproducir el mismo ambiente en otra región o cuenta significa
repetir decenas de clics sin garantía de que el resultado sea idéntico.

La **infraestructura como código** (IaC) resuelve esto describiendo los recursos en un
archivo de texto versionado. El archivo es la fuente de verdad: se revisa en un *pull
request*, se guarda en el repositorio junto al código, y produce siempre el mismo
ambiente. La diferencia es la misma que se vio entre subir archivos a mano y hacer
`git push`: un proceso reproducible en lugar de una serie de pasos manuales.

:::slide
## Click-ops vs. infraestructura como código

**click-ops**: Configurar recursos a mano.

**Infrastructure as Code (IaC)**: describe los recursos en formato de texto versionado.
:::

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
de nivel superior. Solo una es obligatoria: `Resources`. Las demás son opcionales y
aparecen según lo que el template necesite.

:::slide
## Las secciones de un template

{{table-seccion-template}}
:::

{#table-seccion-template}
| Sección | Para qué sirve |
| --- | --- |
| `Resources` | **Obligatoria.** Los recursos a crear: la tabla, el clúster, el balanceador. |
| `Parameters` | Valores que se proveen al lanzar el stack (por ejemplo, el URI de la imagen). |
| `Outputs` | Valores que el stack expone al terminar (por ejemplo, la URL del ALB). |
| `Mappings` | Tablas de búsqueda fijas (por ejemplo, una AMI distinta por región). |
| `Conditions` | Reglas que activan o desactivan recursos según los parámetros. |

En la Semana 1 ya se interactuó con tres de ellas sin saberlo: se completó un **parámetro**
(el URI de la imagen), y se leyó un **output** (la URL del ALB) que el template expuso al
llegar a `CREATE_COMPLETE`.

:::inline-slide light
## Recursos: nombre lógico y nombre físico

Cada recurso dentro de `Resources` tiene un **nombre lógico** (*logical ID*): el
identificador que se le da dentro del template. CloudFormation, al crear el recurso,
le asigna además un **nombre físico**: el identificador real en AWS.

```yaml
Resources:
  TablaApp:                          # nombre lógico (se elige en el template)
    Type: AWS::DynamoDB::Table
    Properties:
      TableName: taller-datos        # nombre físico (opcional)
      BillingMode: PAY_PER_REQUEST
```
:::

El nombre lógico (`TablaApp`) es cómo el resto del template se refiere a este recurso.
El nombre físico (`taller-datos`) es cómo aparece en la consola de DynamoDB. Si no se
especifica un nombre físico, CloudFormation genera uno único automáticamente. Una
práctica habitual, porque evita colisiones de nombres al lanzar el mismo template
varias veces.

:::inline-slide
## Parámetros: tipos y validación

:::skip
Los parámetros son la interfaz del template: los valores que se piden al lanzar el
stack. En la Semana 1 se completó uno (`ImageUri`) sin mirar su definición. Vale la
pena mirarla ahora, porque un parámetro bien declarado hace más que recibir un valor:
lo **valida antes de crear ningún recurso**.
:::

```yaml
Parameters:
  ImageUri:
    Type: String
    Description: URI de la imagen en ECR, con etiqueta.
    AllowedPattern: '^[0-9]{12}\.dkr\.ecr\.[a-z0-9-]+\.amazonaws\.com/[a-z0-9._/-]+:[a-zA-Z0-9._-]+$'
    ConstraintDescription: >-
      Debe ser un URI de ECR con etiqueta, por ejemplo
      123456789012.dkr.ecr.us-east-1.amazonaws.com/taller-aws-maria:latest
```

Además de `Type` y `Description`, la declaración puede incluir:

- **`Default`** — el valor que se usa si el campo se deja sin completar.
- **`AllowedValues`** — restringe el valor a una lista cerrada. Es lo que convierte un
  campo de texto libre en una elección sin ambigüedades:

:::skip
  ```yaml
  RedirigirAHttps:
    Type: String
    Default: "no"
    AllowedValues: ["no", "si"]
  ```
:::

- **`AllowedPattern`** — una expresión regular que el valor debe cumplir. Junto con
  **`ConstraintDescription`** (el mensaje que se muestra cuando no se cumple), es la
  diferencia entre un críptico "Parameter validation failed" y un mensaje que explica
  el formato esperado.
- **`MinLength`/`MaxLength`** y **`MinValue`/`MaxValue`** — límites de longitud (para
  `String`) o de rango (para `Number`).
:::

La validación corre **antes** de crear nada: un URI mal formado se rechaza en el
formulario de la consola, no veinte minutos después con un servicio ECS que no
arranca.

:::inline-slide light
### Tipos específicos de AWS

`Type` no se limita a `String` y `Number`. CloudFormation define tipos que representan
recursos existentes de la cuenta:

```yaml
  # Los tipos AWS::EC2::*::Id hacen que la consola muestre un desplegable con
  # los recursos de la cuenta, en vez de un campo de texto libre.
  VpcId:
    Type: AWS::EC2::VPC::Id
    Description: VPC existente donde se despliega el ambiente.
```
:::

Con `AWS::EC2::VPC::Id`, la consola reemplaza el campo de texto por un **desplegable
con las VPCs reales de la cuenta**, y CloudFormation verifica que el valor exista
antes de lanzar el stack. Quien usó la variante de VPC existente ya los vio en acción:
los campos de VPC y subredes eran desplegables, no texto libre. Otros tipos de la
misma familia: `AWS::EC2::Subnet::Id`, `AWS::EC2::SecurityGroup::Id`, y
`AWS::Route53::HostedZone::Id` (el que usa el stack opcional de HTTPS).

## Funciones

Los recursos rara vez son independientes: el servicio ECS necesita el nombre del
clúster, el ALB necesita el ID de la subred, la *task definition* necesita el URI de
la imagen. Las **funciones** permiten referirse a un valor que solo se
conoce cuando el stack se crea, sin escribirlo a mano.

Las tres más comunes:

:::inline-slide with-title
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
:::

`!Ref` y `!GetAtt` son la forma corta de `Ref` y `Fn::GetAtt`; ambas notaciones son
equivalentes, y las dos aparecen al leer templates de la documentación de AWS. `Ref`
es la única función sin el prefijo `Fn::`; todas las demás lo llevan.

::: extra ¿Por qué YAML y no JSON?
CloudFormation acepta los dos formatos, y son intercambiables. Este taller usa **YAML**
porque admite comentarios (líneas con `#`), es más compacto, y ofrece la sintaxis corta
de las funciones intrínsecas (`!Ref` en lugar de `{ "Ref": "..." }`). JSON sigue siendo
común en templates generados por herramientas. Saber leer ambos es útil; para escribir
a mano, YAML es más cómodo.
:::

### Trabajar con listas

Las tres funciones anteriores devuelven un valor suelto. Otras cuatro trabajan sobre
**listas**, y aparecen en cuanto un template toca la red o construye una cadena a
partir de varias partes.

:::inline-slide with-title light
- **`!Select [índice, lista]`** — devuelve el elemento de la lista en esa posición.
  El índice empieza en cero.
- **`!GetAZs región`** — devuelve la lista de zonas de disponibilidad de una región.
  Con una cadena vacía (`""`) usa la región donde se lanza el stack.

:::skip
Las dos casi siempre viajan juntas. Así elige el stack de red la zona de cada subred,
sin escribir `us-east-1a` en ningún lado:
:::

```yaml
  SubredPublicaA:
    Type: AWS::EC2::Subnet
    Properties:
      AvailabilityZone: !Select [0, !GetAZs ""]    # la primera AZ de la región

  SubredPublicaB:
    Type: AWS::EC2::Subnet
    Properties:
      AvailabilityZone: !Select [1, !GetAZs ""]    # la segunda
```

:::skip
El template queda **portable**: el mismo archivo lanzado en otra región toma las
zonas de esa región. Escribir el nombre de la zona a mano lo ataría a una sola.
:::

- **`!Join [separador, lista]`** — une los elementos de una lista en una sola cadena.

  ```yaml
  Value: !Join ["-", ["taller", !Ref AWS::Region, "app"]]   # taller-us-east-1-app
  ```

:::skip
  Entre `!Join` y `!Sub` la elección es de legibilidad: `!Sub` gana cuando las partes
  son fijas, `!Join` gana cuando la lista es variable o viene de un parámetro.
:::

- **`!Split [separador, cadena]`** — la operación inversa: parte una cadena en una
  lista. Es la forma habitual de recibir varios valores en un solo parámetro de tipo
  `String`.

  ```yaml
  Subnets: !Split [",", !Ref ListaSubredes]    # "subnet-a,subnet-b" → [subnet-a, subnet-b]
  ```
:::

:::inline-slide
## Pseudo Parámetros

:::skip
Algunos valores no los provee quien lanza el stack, ni los crea un recurso: los
aporta CloudFormation. Son los **parámetros pseudo**, y se usan con `!Ref` o dentro
de un `!Sub` igual que cualquier otro parámetro, aunque no se declaren en
`Parameters`.
:::

| Parámetro pseudo | Qué devuelve |
| --- | --- |
| `AWS::Region` | La región donde se lanza el stack (`us-east-1`). |
| `AWS::StackName` | El nombre del stack (`taller-aws-maria-red`). |
| `AWS::AccountId` | El número de doce dígitos de la cuenta. |
| `AWS::Partition` | La partición: `aws`, `aws-cn`, o `aws-us-gov`. Se usa al armar ARNs. |
| `AWS::NoValue` | Un valor especial: la propiedad que lo recibe **desaparece**. |


:::skip
Los dos primeros ya aparecieron sin nombre. `AWS::Region` estaba en el ejemplo de
`!Sub` de más arriba, y `AWS::StackName` es la pieza que hace convivir a varios
participantes en la misma cuenta:
:::

```yaml
    Export:
      Name: !Sub "${AWS::StackName}-vpc-id"    # taller-aws-maria-red-vpc-id
```

:::skip
Un export debe ser único en toda la región. Prefijarlo con el nombre del stack
convierte un nombre que colisionaría entre participantes en uno que no puede
colisionar, sin pedir ningún parámetro extra.

`AWS::Partition` cumple el mismo papel al construir un ARN a mano, donde escribir
`arn:aws:...` funciona en las regiones comerciales y falla en China y en GovCloud:
:::

```yaml
  Resource: !Sub "arn:${AWS::Partition}:s3:::${NombreBucket}/*"
```

`AWS::NoValue` es distinto de los demás: no es un texto, es una instrucción. Junto
con `!If`, es la forma de **omitir una propiedad** en lugar de darle un valor.
:::

:::inline-slide light
## Mappings

:::skip
`Mappings` es una tabla de búsqueda fija, escrita dentro del template: dos niveles de
clave, y un valor. Sirve cuando un valor cambia según la región, el ambiente, o
cualquier otro eje conocido de antemano, y no se quiere pedir como parámetro.
:::

```yaml
Mappings:
  PorAmbiente:
    dev:
      TamanoTarea: "256"
      Tareas: 1
    prod:
      TamanoTarea: "1024"
      Tareas: 3
```

La función **`!FindInMap [mapa, clavePrimaria, claveSecundaria]`** lee la tabla:

```yaml
      Cpu: !FindInMap [PorAmbiente, !Ref Ambiente, TamanoTarea]
```

:::skip
Un mapa es estático: sus valores se escriben en el template y no se pueden calcular.
Ahí está la diferencia con un parámetro. El parámetro pregunta *qué ambiente es*; el
mapa responde *qué implica ese ambiente*. Así un solo template sirve a `dev` y a
`prod` con un único valor de entrada, en vez de seis.

::: extra El caso clásico: una AMI por región
`Mappings` nació para el problema de las AMIs. El ID de una imagen de EC2 cambia en
cada región, así que un template portable necesitaba una tabla de `región → ID`.
Hoy ese caso concreto se resuelve mejor con un parámetro de tipo
`AWS::SSM::Parameter::Value<AWS::EC2::Image::Id>`, que lee el ID vigente desde
Parameter Store en vez de dejarlo congelado en el archivo. `Mappings` sigue siendo
útil para tablas que de verdad son fijas, como la de ambientes de arriba.
:::
:::
:::

:::inline-slide
## Condiciones

:::skip
Un mismo template a veces debe comportarse distinto según sus parámetros: crear un
recurso solo en ciertos casos, o cambiar una propiedad. Para eso existen las
**condiciones**: expresiones booleanas con nombre, definidas en la sección
`Conditions` y evaluadas al lanzar (o actualizar) el stack.
:::

```yaml
Conditions:
  RedirigirHttps: !Equals [!Ref RedirigirAHttps, "si"]
```

Una condición se construye con las funciones lógicas `!Equals`, `!And`, `!Or`, y
`!Not`, y puede referirse a otra condición ya definida con `!Condition`:

```yaml
Conditions:
  CrearGateway: !Equals [!Ref InternetGatewayId, ""]
  GatewayExistente: !Not [!Condition CrearGateway]
```

Una vez definida, la condición se usa de dos maneras:

:::skip
- **Sobre un recurso completo**, con el atributo `Condition` (al lado de `Type`, no
  dentro de `Properties`): el recurso se crea solo si la condición es verdadera.
:::

  ```yaml
  GatewayInternet:
    Type: AWS::EC2::InternetGateway
    Condition: CrearGateway    # solo existe si no se recibió un gateway
  ```

:::skip
- **Dentro de una propiedad**, con la función `!If [condición, valorSi, valorNo]`.
  Así decide el listener HTTP del template de la Semana 1 entre servir la aplicación
  y redirigir a HTTPS:
:::
  ```yaml
  DefaultActions:
    - !If
      - RedirigirHttps
      - Type: redirect
        RedirectConfig:
          Protocol: HTTPS
          Port: "443"
          StatusCode: HTTP_301
      - Type: forward
        TargetGroupArn: !Ref GrupoDestino
  ```

::: warning
Las condiciones solo pueden mirar **parámetros** (y otras condiciones), nunca
recursos: se evalúan antes de crear nada.
:::
:::

:::inline-slide
### Omitir una propiedad con `AWS::NoValue`

:::skip
`!If` obliga a dar dos valores, pero a veces lo que se quiere en una de las ramas no
es *otro valor*, sino **ninguno**: que la propiedad no aparezca, y que el recurso use
el valor por defecto de AWS. Para eso está `AWS::NoValue`:
:::

```yaml
  TablaApp:
    Type: AWS::DynamoDB::Table
    Properties:
      TableName: !If [NombreFijo, !Ref NombreTabla, !Ref "AWS::NoValue"]
```

Si la condición es falsa, la propiedad `TableName` se borra del template antes
de crear nada, y CloudFormation genera el nombre físico como si nunca se
hubiera escrito. Dar `""` en su lugar sería distinto (y en algunos casso un
error). Sería pedir una tabla llamada *cadena vacía*.
:::

:::inline-slide
## Leer la definición de un recurso

:::skip
El template dice `Type: AWS::DynamoDB::Table` y debajo una lista de propiedades. ¿De
dónde sale esa lista? Cada tipo de recurso tiene su página en la
[referencia oficial de recursos de CloudFormation](https://docs.aws.amazon.com/AWSCloudFormation/latest/UserGuide/aws-template-resource-type-ref.html).
La forma más rápida de llegar es buscar el tipo textual (por ejemplo,
`AWS::ECS::Service`) en un buscador: el primer resultado es casi siempre la página
oficial.
:::

:::add
[Referencia Oficial de CloudFormation](https://docs.aws.amazon.com/AWSCloudFormation/latest/UserGuide/aws-template-resource-type-ref.html).
:::

En cada página conviene leer tres cosas:

:::skip
- **Properties** — todas las propiedades del recurso, y para cada una: si es
  obligatoria (*Required*), su tipo, y ***Update requires***: qué pasa con el recurso
  si esa propiedad cambia en una actualización (se modifica en el lugar, o se
  reemplaza por uno nuevo). Esta última columna se vuelve central en la próxima
  sesión, al actualizar stacks.
- **Return values** — qué devuelve `!Ref` para este recurso, y qué atributos ofrece
  `!GetAtt`. No hay regla general: `!Ref` sobre la tabla de DynamoDB devuelve su
  **nombre**, pero sobre el balanceador devuelve su **ARN**. La página del recurso es
  la única fuente confiable.
- **Examples** — fragmentos YAML y JSON listos para adaptar.
:::

:::add visibility=slide
- **Properties** — todas las propiedades del recurso, si es
  obligatoria, su tipo, y ***si requiere update.***
- **Return values** — qué devuelve `!Ref` para este recurso, y qué atributos ofrece
  `!GetAtt`.
- **Examples** — fragmentos YAML y JSON listos para adaptar.
::: # add
::: # inline-slide

Con esto, leer un recurso desconocido deja de ser un acto de fe: cada propiedad del
template se puede contrastar contra su definición oficial.

:::inline-slide light
## Secretos: nunca en texto plano

:::skip
Un template se versiona en `git`, se comparte, y se puede leer desde la consola. Todo
valor escrito en él es público para cualquiera con acceso al repositorio o al stack.
Por eso la regla es absoluta: **un secreto nunca va en texto plano en un template**
—ni como propiedad, ni como valor por defecto de un parámetro.

La solución es guardar el secreto en **AWS Secrets Manager** y que el template lo
referencie sin contenerlo. Hay dos mecanismos:
:::

- **Referencias dinámicas** — la cadena `{{resolve:secretsmanager:...}}` dentro de
  una propiedad. CloudFormation la resuelve al momento de desplegar; el template solo
  guarda la referencia:

  ```yaml
  MasterUserPassword: '{{resolve:secretsmanager:credenciales-db:SecretString:password}}'
  ```

:::skip
- **`Secrets` + `ValueFrom`** (contenedores) — para variables de entorno de una *task
  definition*, la referencia dinámica no alcanza: el valor resuelto quedaría escrito
  en la task definition, visible para cualquiera que pueda leerla. La propiedad
  `Secrets` guarda solo el **ARN** del secreto, y es ECS quien lo resuelve al
  arrancar cada tarea:
:::

:::add visibility=slide
- **`Secrets` + `ValueFrom`** (contenedores) - la propiedad
  `Secrets` guarda solo el **ARN** del secreto, y es ECS quien lo resuelve al
  arrancar cada tarea:
:::

  ```yaml
  Secrets:
    - Name: CB_APPS_SECRET
      ValueFrom: !Ref SecretoApps   # el ARN del secreto, nunca su valor
  ```

:::skip
Esta es la razón por la que el template de la Semana 1 no define la variable
`CB_APPS_SECRET`: su lugar no es el template. Se agrega más adelante, vía Secrets
Manager, y la sesión sobre contenedores muestra el mecanismo completo, incluido el
permiso de IAM que necesita ECS para leer el secreto.
::: # skip
::: # inline-slide

:::inline-slide
## Outputs: publicar valores, conectar templates

:::skip
La sección `Outputs` declara los valores que el stack expone al terminar. Es lo que
llena la pestaña **Outputs** de la consola, de donde se copió la URL del ALB en la
Semana 1:
:::

```yaml
Outputs:
  ALBUrl:
    Description: URL pública de la aplicación
    Value: !Sub "http://${BalanceadorApp.DNSName}"
```

Cada output tiene un nombre lógico, un `Value` (casi siempre construido con `!Ref`,
`!GetAtt`, o `!Sub`), y una `Description` opcional que la consola muestra al lado.
:::

:::inline-slide
### Exports: el contrato entre stacks

Un output puede además **exportarse**: publicarse con un nombre único en la región,
visible para cualquier otro stack de la cuenta:

```yaml
  ALBArn:
    Description: ARN del balanceador, para agregarle listeners desde otro stack
    Value: !Ref BalanceadorApp
    Export:
      Name: !Sub "${AWS::StackName}-alb-arn"
```

Otro template consume ese valor con la función `Fn::ImportValue`:

```yaml
  ListenerHTTPS:
    Type: AWS::ElasticLoadBalancingV2::Listener
    Properties:
      LoadBalancerArn:
        Fn::ImportValue: !Sub "${AppStackName}-alb-arn"
```
:::

Así funciona el stack opcional de HTTPS de la Semana 1: agrega un listener 443 al
balanceador de otro stack **sin modificarlo**, leyendo sus exports. Quien lo desplegó
también vivió la otra cara del mecanismo: mientras un stack importa un export,
CloudFormation **bloquea el borrado** del stack que lo publica. El export no es un
dato suelto: es un contrato entre stacks, y el contrato se hace cumplir.

Este mecanismo es la base de la última parte de la semana: separar el ambiente en
stacks de **red**, **datos**, y **aplicación**, conectados exclusivamente por exports
e imports.
