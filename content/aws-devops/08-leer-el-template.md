+++
title = "Leer el template paso a paso"
+++

## El template de la Semana 1, por dentro

Ya se conocen las secciones de un template. Ahora se recorre el archivo real que desplegó
la aplicación, `taller-aws-devops-semana1.yaml`, recurso por recurso, conectando cada bloque con lo
que se vio en la consola la semana pasada.

:::inline-slide light
## Analizando un template

:::app
<cb-file path="./infra/templates/taller-aws-devops-semana1-vpc-existente.yaml" type="yaml" toggleable full-path></cb-file>
:::

:::app
<cb-file path="./infra/templates/taller-aws-devops-extra-https.yaml" type="yaml" toggleable full-path></cb-file>
:::

:::


## El encabezado y los parámetros

El template comienza declarando su versión y los parámetros que recibe al lanzarse:

```yaml
AWSTemplateFormatVersion: "2010-09-09"
Description: Taller AWS DevOps — Semana 1

Parameters:
  ImageUri:
    Type: String
    Description: URI de la imagen en ECR, con etiqueta.
```

Aquí está el parámetro que se completó al crear el stack: `ImageUri`. Es el único
valor que el template no puede conocer por sí solo, ya que depende de la imagen que el build
publicó en ECR, y por eso se pide al lanzarlo.

## Los atributos de un recurso

Antes de recorrer los recursos conviene fijar una distinción que confunde a menudo.
Dentro de un recurso hay dos niveles de información:

- **`Properties`** — la configuración del recurso en AWS: el tipo de instancia, el
  puerto, el modo de facturación. Es lo que se documenta en la página del tipo.
- **Los atributos** — instrucciones para **CloudFormation**, no para el servicio.
  Van al mismo nivel que `Type`, **fuera** de `Properties`.

Escribir un atributo dentro de `Properties` es el error de sintaxis más común al
empezar: CloudFormation lo rechaza como propiedad inválida del recurso.

{#table-atributos-recurso}
| Atributo | Para qué sirve |
| --- | --- |
| `Type` | **Obligatorio.** El tipo de recurso: `AWS::ECS::Service`. |
| `Properties` | La configuración del recurso. Obligatoria salvo que el tipo no tenga propiedades requeridas. |
| `DependsOn` | Fuerza a crear otro recurso antes que este. |
| `Condition` | El recurso se crea solo si la condición es verdadera. |
| `DeletionPolicy` | Qué hacer con el recurso al borrar el stack: borrarlo, conservarlo, o copiarlo. |
| `UpdateReplacePolicy` | Lo mismo, pero cuando una actualización obliga a reemplazarlo. |
| `Metadata` | Datos libres para herramientas y para quien lee. CloudFormation los ignora. |

```yaml
  ServicioApp:
    Type: AWS::ECS::Service       # atributo
    DependsOn: ListenerHTTP       # atributo
    Properties:                   # atributo, y dentro de él la configuración
      DesiredCount: 1
```

Este recorrido usa `DependsOn` y `Condition`. Las dos políticas se retoman en la
sección de buenas prácticas, donde protegen los datos de la tabla.

:::slide
## Los atributos de un recurso

{{table-atributos-recurso}}

Van al lado de `Type`, **nunca dentro de `Properties`**.
:::

## La tabla de DynamoDB

```yaml
Resources:
  TablaApp:
    Type: AWS::DynamoDB::Table
    Properties:
      BillingMode: PAY_PER_REQUEST
      AttributeDefinitions:
        - AttributeName: collection
          AttributeType: S
        - AttributeName: key
          AttributeType: S
      KeySchema:
        - AttributeName: collection
          KeyType: HASH
        - AttributeName: key
          KeyType: RANGE
```

Este es un servicio `Stateful`. Vamos a ver más adelante porque mezclar sistemas
`Stateful` y `Stateless` no es la mejor opción.

::: info
Note que no tiene `TableName`: CloudFormation le asignó un nombre físico
generado, lo que permite lanzar el template varias veces sin colisiones.
:::

## El clúster y el servicio ECS

```yaml
  ClusterApp:
    Type: AWS::ECS::Cluster

  ServicioApp:
    Type: AWS::ECS::Service
    DependsOn: ListenerHTTP
    Properties:
      Cluster: !Ref ClusterApp
      DesiredCount: 1
      LaunchType: FARGATE
      TaskDefinition: !Ref TareaApp
```

Aquí aparece la primera función importante: `Cluster: !Ref ClusterApp`
conecta el servicio con el clúster creado más arriba, sin escribir su nombre a mano.
Lo mismo `TaskDefinition: !Ref TareaApp`. Estos son los recursos que se vieron en
**ECS → Clusters**: el clúster, y dentro de él el servicio con su tarea en estado
`RUNNING`.

### El orden de creación: implícito y explícito

Un template no dice en qué orden crear los recursos, y el orden del archivo no
importa. CloudFormation lo **deduce** de las referencias: si el servicio hace
`!Ref ClusterApp`, el clúster tiene que existir primero. Esa es la **dependencia
implícita**, y cubre la mayoría de los casos. Por eso los recursos sin relación entre
sí se crean en paralelo, y por eso el orden de la pestaña **Events** no sigue al
archivo.

El problema aparece cuando la dependencia **existe en la realidad pero no en el
template**: dos recursos que no se referencian, y aun así uno necesita al otro. Para
esos casos está `DependsOn`, que declara el orden a mano.

Es exactamente el caso de `DependsOn: ListenerHTTP`. El servicio no menciona el
listener en ninguna propiedad: se conecta al *target group* (`TargetGroupArn: !Ref
GrupoDestino`), y el listener es un tercero que también apunta a ese target group.
Para CloudFormation son dos recursos sin relación, y los crearía en paralelo.

Pero ECS se niega a crear un servicio cuyo target group todavía no esté asociado a un
balanceador, y quien hace esa asociación es justamente el listener. Sin `DependsOn`, el
stack falla de manera intermitente —a veces gana la carrera, a veces no— con un error
del tipo *"The target group ... does not have an associated load balancer"*. El
atributo elimina la carrera.

El template usa el mismo atributo una segunda vez, en el recurso de red:

```yaml
  RutaSalida:
    Type: AWS::EC2::Route
    DependsOn: VinculoGateway     # el gateway debe estar adjunto a la VPC
    Properties:
      DestinationCidrBlock: 0.0.0.0/0
      GatewayId: !Ref GatewayInternet
```

La ruta referencia al *gateway*, pero no al **vínculo** que lo adjunta a la VPC. Sin
ese vínculo, crear la ruta falla. `!Ref GatewayInternet` no alcanza para expresarlo,
y `DependsOn` sí.

La regla práctica: **no agregar `DependsOn` por las dudas**. Cada uno que sobra
serializa lo que podría ir en paralelo, y alarga el despliegue. Se agrega cuando hay
una dependencia real que ninguna referencia expresa —y conviene dejar un comentario
diciendo cuál es, porque no se deduce del código.

:::slide
## El orden de creación

- **Implícito** — `!Ref` y `!GetAtt` ya declaran el orden. Cubre casi todo.
- **Explícito** — `DependsOn`, para dependencias reales que ninguna referencia
  expresa.

Lo que no depende de nada se crea **en paralelo**: por eso **Events** no sigue el
orden del archivo.

`DependsOn` de más serializa el despliegue.
:::

## La red: subredes, zonas, y etiquetas

El bloque de red usa las funciones sobre listas de la sección anterior:

```yaml
  SubredPublicaA:
    Type: AWS::EC2::Subnet
    Properties:
      VpcId: !Ref VpcApp
      CidrBlock: 10.0.0.0/24
      AvailabilityZone: !Select [0, !GetAZs ""]
      MapPublicIpOnLaunch: true
      Tags:
        - Key: Name
          Value: !Sub "${AWS::StackName}-publica-a"
```

`!Select [0, !GetAZs ""]` toma la primera zona de disponibilidad de la región actual,
y la subred B toma la segunda con el índice `1`. Dos subredes en dos zonas distintas
es lo que le permite al balanceador sobrevivir a la caída de una zona: es el mínimo
que exige un ALB.

Y aquí aparecen las **etiquetas**. `Tags` es una lista de pares `Key`/`Value` que se
adjunta al recurso. La etiqueta `Name` es la que la consola de EC2 muestra como
nombre en sus listados —sin ella, la subred aparece con su ID (`subnet-0a1b2c…`) y
nada más. Construirla con `!Sub "${AWS::StackName}-publica-a"` hace que el nombre
diga a qué stack pertenece, lo que importa cuando varios participantes comparten la
cuenta.

::: info
Las etiquetas no son decoración. AWS las usa para **repartir costos** (Cost
Explorer agrupa por etiqueta), para **filtrar** recursos en la consola y en el CLI, y
para dar o negar permisos en IAM según el valor de una etiqueta. La sección de buenas
prácticas retoma el tema con las etiquetas a nivel de stack.
:::

## La task definition y la imagen

```yaml
  TareaApp:
    Type: AWS::ECS::TaskDefinition
    Properties:
      RequiresCompatibilities: [FARGATE]
      Cpu: "256"
      Memory: "512"
      ContainerDefinitions:
        - Name: app
          Image: !Ref ImageUri
          PortMappings:
            - ContainerPort: 8080
```

Esta es la pieza clave: `Image: !Ref ImageUri` es donde el URI pegado al lanzar
el stack se convierte en la imagen que ejecuta el contenedor. El parámetro del
encabezado llega hasta aquí a través de una sola función intrínseca.

## El balanceador y la salida

```yaml
  BalanceadorApp:
    Type: AWS::ElasticLoadBalancingV2::LoadBalancer
    Properties:
      Type: application

Outputs:
  ALBUrl:
    Description: URL pública de la aplicación
    Value: !Sub "http://${BalanceadorApp.DNSName}"
```

El output `ALBUrl` es el valor que se copió de la pestaña **Outputs**. La función `!Sub`
construye la URL completa insertando el atributo `DNSName` del balanceador —un valor que
solo existe una vez que AWS crea el ALB.

::: extra El template completo tiene más recursos
Por claridad, los bloques anteriores omiten los grupos de seguridad, los roles de IAM,
el *listener* del ALB, y el *target group*. El archivo real los incluye: cada uno
aparece como un recurso más en `Resources`, conectado a los demás con `!Ref` y
`!GetAtt`. En la sección de buenas prácticas se verá cómo navegar un template largo
sin perderse.
:::

::: extra Ordenar el formulario de la consola con Metadata
El stack de aplicación de la próxima sesión pide nueve parámetros, y la consola los
muestra en el orden en que aparecen en el template, con el nombre lógico como
etiqueta. La sección `Metadata` permite mejorar eso sin tocar ningún recurso:

```yaml
Metadata:
  AWS::CloudFormation::Interface:
    ParameterGroups:
      - Label: { default: "Imagen de la aplicación" }
        Parameters: [ImageUri, ComandoContenedor]
      - Label: { default: "Stacks de los que depende" }
        Parameters: [RedStackName, DatosStackName, PlataformaStackName]
      - Label: { default: "Lugar en el balanceador" }
        Parameters: [NombreHost, RutaPath, Prioridad, UsarHttps]
    ParameterLabels:
      ImageUri: { default: "URI de la imagen en ECR" }
```

`ParameterGroups` agrupa los parámetros en secciones con título, y `ParameterLabels`
reemplaza el nombre lógico por un texto legible. Solo lo lee la consola de
CloudFormation: el CLI y las APIs lo ignoran, y el stack se comporta igual sin él. Es
puro cuidado por quien va a completar el formulario.
:::
