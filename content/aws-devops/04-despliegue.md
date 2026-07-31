+++
title = "El despliegue: CloudFormation como caja negra"
+++

## El salto de la imagen a la aplicación

En la sección anterior se construyó y publicó la imagen Docker en ECR. Pero una imagen
en un registro no es una aplicación en línea: alguien tiene que lanzar el contenedor,
colocarlo detrás de un balanceador de carga, conectarlo a una base de datos, y
configurar la red. Hacer eso paso a paso desde la consola tomaría más de una hora y
sería difícil de reproducir exactamente.

**AWS CloudFormation** resuelve este problema con un enfoque declarativo: se describe
en un archivo YAML o JSON qué recursos se quieren: un clúster ECS, un servicio Fargate,
un ALB, una tabla DynamoDB, grupos de seguridad, roles de IAM. Luego CloudFormation los
crea todos en el orden correcto, manejando las dependencias automáticamente.

## El template de esta semana

El instructor provee el archivo
`./infra/templates/taller-aws-devops-semana1.yaml`. Este template es, por
ahora, una **caja negra**: se lanza con dos parámetros y se obtiene un ambiente
funcional en minutos. No es necesario entender su contenido todavía. Esto lo
veremos durante la Semana 2.

Lo que despliega el template:

- Una **tabla de DynamoDB** para la aplicación.
- Un **clúster ECS** y un **servicio Fargate** que ejecuta la imagen Docker.
- Un **Application Load Balancer** (ALB) que recibe el tráfico HTTP y lo dirige al
  contenedor.
- Los roles de IAM, grupos de seguridad, y configuración de red necesarios para que
  todo funcione junto.

Los únicos parámetros configurables son el **nombre del stack** y el **URI de la
imagen en ECR**. El resto lo gestiona el template.

## Un detalle que vale la pena notar

Al abrir la URL del ALB en el navegador, se ve cargarse esta misma guía: la
plataforma del taller servida desde el despliegue en ECS. La aplicación que
se construyó, desplegó, y opera es exactamente el entorno desde el que se leen estas
instrucciones. No es un ejemplo genérico —es el sistema real.

:::slide
## La caja negra de la Semana 1

Un template, dos parámetros, un ambiente completo:

- DynamoDB · ECS + Fargate · Application Load Balancer
- Roles de IAM, grupos de seguridad, red

Los parámetros configurables son el **nombre del stack** y el **URI de la imagen**.
:::

## Práctica guiada: lanzar el stack de CloudFormation

### Abrir CloudFormation

1. Abrir [**CloudFormation**](https://console.aws.amazon.com/cloudformation/home).
2. Confirmar que la región seleccionada (esquina superior derecha) es la misma que se ha
   usado para todos los recursos del taller.

### Crear el stack

1. Pulsar **Create stack** y seleccionar **With new resources (standard)**.
2. En la sección **Specify template**, seleccionar **Upload a template file**.
3. Pulsar **Choose file** y seleccionar el archivo `taller-aws-devops-semana1.yaml` provisto por
   el instructor.
4. Pulsar **Next**.

### Completar los parámetros

1. En **Stack name**, escribir `taller-<su-nombre>` (por ejemplo: `taller-maria`). El
   nombre del stack identifica el ambiente en la consola y debe ser único en la región.
2. En los parámetros del template, localizar el campo correspondiente al **URI de
   la imagen**. Pegar el URI completo copiado de ECR al final de la sección anterior.
   El formato es:
   ```
   123456789012.dkr.ecr.us-east-1.amazonaws.com/taller-aws-<su-nombre>:latest
   ```
1. Revisar los demás parámetros. Dejarlos con sus valores predeterminados a menos que el
   instructor indique lo contrario.
2. Pulsar **Next**.

### Configurar opciones del stack

1. En la pantalla de opciones, no es necesario cambiar nada. Pulsar **Next**.

### Confirmar y lanzar

1. En la pantalla de revisión, desplazarse hasta la sección **Capabilities** al pie
    de la página. Se verá un aviso sobre que el template puede crear recursos de IAM.
    Marcar la casilla **I acknowledge that AWS CloudFormation might create IAM
    resources**.
2. Pulsar **Submit** (o **Create stack**, según la versión de la consola).

### Seguir la creación del stack

1. CloudFormation lleva automáticamente a la vista del stack recién iniciado.
    Seleccionar la pestaña **Events**. Se ve cómo se van creando los recursos en tiempo
    real, uno por uno, con su estado.
2. Esperar hasta que el estado del stack (en la parte superior) cambie a
    **CREATE_COMPLETE**. El proceso toma entre 3 y 8 minutos, dependiendo de la región.
    Si algún recurso falla, el estado cambia a **ROLLBACK_IN_PROGRESS** y CloudFormation
    deshará los cambios automáticamente —revisar el evento fallido para entender el motivo.

### Obtener la URL de la aplicación

1. Una vez en **CREATE_COMPLETE**, seleccionar la pestaña **Outputs**.
2. Se verá una salida llamada `ALBUrl` (o similar, según el template). Copiar el valor
    —es la URL pública del Application Load Balancer.
3. Abrir esa URL en una nueva pestaña del navegador. En unos segundos se ve cargarse
    esta guía del taller, servida desde el contenedor recién desplegado en ECS.

---

{#ejercicio-7}
### Ejercicio 7 — Desplegar la aplicación

Lanzar el stack de CloudFormation con el template `taller-aws-devops-semana1.yaml` provisto por
el instructor. Usar como URI de la imagen el valor copiado de ECR al terminar el
Ejercicio 4. Al terminar, abrir la URL del ALB en el navegador y confirmar que la
aplicación está en línea.

::: solucion
1. En la consola de AWS, abrir [**CloudFormation**](https://console.aws.amazon.com/cloudformation/home).
2. Pulsar **Create stack → With new resources (standard)**.
3. Seleccionar **Upload a template file**, pulsar **Choose file**, y subir
   `taller-aws-devops-semana1.yaml`.
4. Pulsar **Next**.
5. En **Stack name**, escribir `taller-<su-nombre>`.
6. En el campo del URI de la imagen, pegar el URI completo de ECR con la etiqueta
   `latest` (por ejemplo:
   `123456789012.dkr.ecr.us-east-1.amazonaws.com/taller-aws-maria:latest`).
7. Pulsar **Next** dos veces para llegar a la pantalla de revisión.
8. En la sección **Capabilities**, marcar la casilla de aceptación de recursos de IAM.
9. Pulsar **Submit**.
10. En la pestaña **Events**, esperar a que el estado del stack llegue a
    **CREATE_COMPLETE**.
11. En la pestaña **Outputs**, copiar el valor de **ALBUrl**.
12. Abrir esa URL en el navegador. Se verá la plataforma del taller corriendo desde el
    despliegue.
:::

:::slide light
{{ejercicio-7}}
:::
