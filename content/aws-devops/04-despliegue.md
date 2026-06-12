+++
title = "El despliegue — CloudFormation como caja negra"
+++

## El salto de la imagen a la aplicación

En la sección anterior construyó y publicó la imagen Docker en ECR. Pero una imagen
en un registro no es una aplicación en línea: alguien tiene que lanzar el contenedor,
colocarlo detrás de un balanceador de carga, conectarlo a una base de datos, y
configurar la red. Hacer eso paso a paso desde la consola tomaría más de una hora y
sería difícil de reproducir exactamente.

**AWS CloudFormation** resuelve este problema con un enfoque declarativo: usted describe
en un archivo YAML o JSON qué recursos quiere —un clúster ECS, un servicio Fargate,
un ALB, una tabla DynamoDB, grupos de seguridad, roles de IAM— y CloudFormation los
crea todos en el orden correcto, manejando las dependencias automáticamente.

## La plantilla de esta semana

El instructor le proveyó el archivo `taller-semana1.yaml`. Esta plantilla es, por ahora,
una **caja negra**: usted la lanza con dos parámetros y obtiene un ambiente funcional
en minutos. No necesita entender su contenido esta semana —eso es el tema de la Semana 2.

Lo que despliega la plantilla:

- Una **tabla de DynamoDB** para la aplicación.
- Un **clúster ECS** y un **servicio Fargate** que ejecuta su imagen Docker.
- Un **Application Load Balancer** (ALB) que recibe el tráfico HTTP y lo dirige al
  contenedor.
- Los roles de IAM, grupos de seguridad, y configuración de red necesarios para que
  todo funcione junto.

Los únicos parámetros que usted controla son el **nombre del stack** y el **URI de la
imagen en ECR**. El resto lo gestiona la plantilla.

## Un detalle que vale la pena notar

Cuando abra la URL del ALB en el navegador, verá cargarse esta misma guía: la
plataforma del taller servida desde su propio despliegue en ECS. La aplicación que
usted construyó, desplegó, y opera es exactamente el entorno desde el que lee estas
instrucciones. No es un ejemplo genérico —es el sistema real.

## Práctica guiada: lanzar el stack de CloudFormation

### Abrir CloudFormation

1. En la barra de búsqueda de la consola de AWS, escriba `CloudFormation` y ábralo.
2. Confirme que la región seleccionada (esquina superior derecha) es la misma que ha
   usado para todos los recursos del taller.

### Crear el stack

3. Pulse **Create stack** y seleccione **With new resources (standard)**.
4. En la sección **Specify template**, seleccione **Upload a template file**.
5. Pulse **Choose file** y seleccione el archivo `taller-semana1.yaml` provisto por
   el instructor.
6. Pulse **Next**.

### Completar los parámetros

7. En **Stack name**, escriba `taller-<su-nombre>` (por ejemplo: `taller-maria`). El
   nombre del stack identifica su ambiente en la consola y debe ser único en la región.
8. En los parámetros de la plantilla, localice el campo correspondiente al **URI de
   la imagen**. Pegue el URI completo que copió de ECR al final de la sección anterior.
   El formato es:
   ```
   123456789012.dkr.ecr.us-east-1.amazonaws.com/taller-aws-<su-nombre>:latest
   ```
9. Revise los demás parámetros. Déjelos con sus valores predeterminados a menos que el
   instructor indique lo contrario.
10. Pulse **Next**.

### Configurar opciones del stack

11. En la pantalla de opciones, no es necesario cambiar nada. Pulse **Next**.

### Confirmar y lanzar

12. En la pantalla de revisión, desplácese hasta la sección **Capabilities** al pie
    de la página. Verá un aviso sobre que la plantilla puede crear recursos de IAM.
    Marque la casilla **I acknowledge that AWS CloudFormation might create IAM
    resources with custom names**.
13. Pulse **Submit** (o **Create stack**, según la versión de la consola).

### Seguir la creación del stack

14. CloudFormation lo lleva automáticamente a la vista del stack recién iniciado.
    Seleccione la pestaña **Events**. Verá cómo se van creando los recursos en tiempo
    real, uno por uno, con su estado.
15. Espere hasta que el estado del stack (en la parte superior) cambie a
    **CREATE_COMPLETE**. El proceso toma entre 3 y 8 minutos, dependiendo de la región.
    Si algún recurso falla, el estado cambia a **ROLLBACK_IN_PROGRESS** y CloudFormation
    deshará los cambios automáticamente —revise el evento fallido para entender el motivo.

### Obtener la URL de la aplicación

16. Una vez en **CREATE_COMPLETE**, seleccione la pestaña **Outputs**.
17. Verá una salida llamada `ALBUrl` (o similar, según la plantilla). Copie el valor
    —es la URL pública del Application Load Balancer.
18. Abra esa URL en una nueva pestaña del navegador. En unos segundos verá cargarse
    esta guía del taller, servida desde el contenedor que acaba de desplegar en su
    propio ECS.

---

### Ejercicio 5 — Despliegue la aplicación

Lance el stack de CloudFormation con la plantilla `taller-semana1.yaml` provista por
el instructor. Use como URI de la imagen el valor que copió de ECR al terminar el
Ejercicio 4. Al terminar, abra la URL del ALB en el navegador y confirme que la
aplicación está en línea.

::: solucion
1. En la consola de AWS, abra **CloudFormation**.
2. Pulse **Create stack → With new resources (standard)**.
3. Seleccione **Upload a template file**, pulse **Choose file**, y suba
   `taller-semana1.yaml`.
4. Pulse **Next**.
5. En **Stack name**, escriba `taller-<su-nombre>`.
6. En el campo del URI de la imagen, pegue el URI completo de ECR con la etiqueta
   `latest` (por ejemplo:
   `123456789012.dkr.ecr.us-east-1.amazonaws.com/taller-aws-maria:latest`).
7. Pulse **Next** dos veces para llegar a la pantalla de revisión.
8. En la sección **Capabilities**, marque la casilla de aceptación de recursos de IAM.
9. Pulse **Submit**.
10. En la pestaña **Events**, espere a que el estado del stack llegue a
    **CREATE_COMPLETE**.
11. En la pestaña **Outputs**, copie el valor de **ALBUrl**.
12. Abra esa URL en el navegador. Verá la plataforma del taller corriendo desde su
    propio despliegue.
:::
