+++
title = "Leer el template paso a paso"
+++

## El template de la Semana 1, por dentro

Ya se conocen las secciones de un template. Ahora se recorre el archivo real que desplegó
la aplicación, `taller-semana1.yaml`, recurso por recurso, conectando cada bloque con lo
que se vio en la consola la semana pasada.

Abrir el archivo `taller-semana1.yaml` en el editor de texto —es el mismo que se subió a
CloudFormation. Se leerá de arriba hacia abajo.

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
valor que el template no puede conocer por sí solo —depende de la imagen que el build
publicó en ECR— y por eso se pide al lanzarlo.

## La tabla de DynamoDB

```yaml
Resources:
  TablaApp:
    Type: AWS::DynamoDB::Table
    Properties:
      BillingMode: PAY_PER_REQUEST
      AttributeDefinitions:
        - AttributeName: id
          AttributeType: S
      KeySchema:
        - AttributeName: id
          KeyType: HASH
```

Este es el recurso que se vio en **DynamoDB → Tables**. Note que no tiene `TableName`:
CloudFormation le asignó un nombre físico generado, lo que permite lanzar el template
varias veces sin colisiones.

## El clúster y el servicio ECS

```yaml
  ClusterApp:
    Type: AWS::ECS::Cluster

  ServicioApp:
    Type: AWS::ECS::Service
    Properties:
      Cluster: !Ref ClusterApp
      DesiredCount: 1
      LaunchType: FARGATE
      TaskDefinition: !Ref TareaApp
```

Aquí aparece la primera función intrínseca importante: `Cluster: !Ref ClusterApp`
conecta el servicio con el clúster creado más arriba, sin escribir su nombre a mano.
Lo mismo `TaskDefinition: !Ref TareaApp`. Estos son los recursos que se vieron en
**ECS → Clusters**: el clúster, y dentro de él el servicio con su tarea en estado
`RUNNING`.

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
el *listener* del ALB, el *target group*, y la configuración de red (subredes, VPC). El
archivo real los incluye: cada uno aparece como un recurso más en `Resources`, conectado
a los demás con `!Ref` y `!GetAtt`. En la sección de buenas prácticas se verá cómo navegar
un template largo sin perderse.
:::

---

{#ejercicio-7}
### Ejercicio 7 — Seguir el rastro de la imagen

Abrir `taller-semana1.yaml` y seguir el recorrido del URI de la imagen: desde el parámetro
que lo recibe, hasta el recurso que finalmente lo usa para ejecutar el contenedor.
Identificar el nombre del parámetro, la función intrínseca que lo transporta, y el
recurso de destino.

::: solucion
1. En la sección `Parameters`, el parámetro se llama **`ImageUri`** (tipo `String`).
   Es el valor que se pegó al crear el stack.
2. En la sección `Resources`, buscar el recurso de tipo
   `AWS::ECS::TaskDefinition` (llamado `TareaApp`).
3. Dentro de `ContainerDefinitions`, la propiedad **`Image: !Ref ImageUri`** es donde
   el parámetro se usa. La función **`!Ref`** devuelve el valor del parámetro y lo
   asigna como imagen del contenedor.
4. El recorrido completo es: `Parameters.ImageUri` → `!Ref ImageUri` →
   `TareaApp.ContainerDefinitions[0].Image`. Un único `!Ref` conecta lo que se
   escribió al lanzar el stack con la imagen que ejecuta Fargate.
:::

:::slide light
{{ejercicio-7}}
:::
