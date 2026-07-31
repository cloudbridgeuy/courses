+++
title = "Destruir, y recrear, el ambiente"
+++

## Por qué se practica la destrucción desde el primer día

En un sistema operado manualmente, un error puede dejar el ambiente en un estado
inconsistente difícil de diagnosticar y corregir. El tiempo de recuperación depende de
cuánto se recuerde de cómo se construyó originalmente, y de si esa memoria es precisa.

Cuando el ambiente se define como código —en este caso, un template de CloudFormation—
la situación cambia radicalmente. Si algo sale mal, la corrección no es reconstruir desde
la memoria: es borrar y volver a crear. El proceso es el mismo que se siguió hace unos
minutos, tarda lo mismo, y produce exactamente el mismo resultado. El **costo de un
error se convierte en minutos de espera**, no en horas de diagnóstico.

Esta es la razón por la que se practica el ciclo completo de destrucción y recreación en
la Semana 1: para que en las semanas siguientes, si algo falla, la respuesta refleja no
sea la urgencia sino la calma. Se borra, se recrea, se sigue.

## Qué sobrevive a la destrucción del stack

Antes de borrar el stack, es útil entender qué destruye CloudFormation y qué no.

El template `taller-semana1.yaml` crea y gestiona: el clúster ECS, el servicio Fargate,
el ALB, la tabla de DynamoDB, los roles de IAM, y la configuración de red. Todos esos
recursos **se eliminan** cuando se borra el stack.

Lo que **no** forma parte del stack y por lo tanto **sobrevive**:

- El **repositorio de CodeCommit** con todo el historial de commits.
- El **repositorio de ECR** con la imagen Docker publicada.
- El **proyecto de CodeBuild** configurado en la sección anterior.

Esto significa que al recrear el stack basta con volver a proporcionar el URI de la
imagen en ECR: el ambiente completo se reconstituye en minutos, sin volver a hacer el
build ni resubir el código.

:::slide
## El seguro del taller

Borrar y recrear el stack devuelve el ambiente a un estado conocido en minutos.

**Sobrevive** (fuera del stack): el repo de CodeCommit, la imagen en ECR, el proyecto
de CodeBuild.
**Se borra** (lo gestiona el stack): ECS, Fargate, ALB, DynamoDB, IAM, red.
:::

## Práctica guiada: borrar el stack

### Iniciar la eliminación

1. En la consola de AWS, abrir [**CloudFormation**](https://console.aws.amazon.com/cloudformation/home).
2. En la lista de stacks, seleccionar el stack `taller-<su-nombre>`.
3. Pulsar **Delete**.
4. En el diálogo de confirmación, pulsar **Delete stack**.

### Seguir los eventos de borrado

1. CloudFormation comienza a eliminar los recursos en orden inverso al de creación
   (primero los que dependen de otros, luego los recursos base). La pestaña **Events**
   muestra cada eliminación en tiempo real.
2. Esperar hasta que el stack desaparezca de la lista o, si la consola lo muestra,
   hasta que el estado sea **DELETE_COMPLETE**. El proceso toma entre 3 y 6 minutos.

> **Nota:** si algún recurso no puede eliminarse automáticamente (por ejemplo, una
> tabla de DynamoDB con protección contra eliminación, o un bucket de S3 con objetos),
> el estado cambiará a **DELETE_FAILED** y el evento fallido indicará el recurso y el
> motivo. El instructor indicará cómo proceder en ese caso.

### Confirmar que la aplicación ya no está en línea

1. Intentar abrir de nuevo la URL del ALB usada antes. El navegador debe mostrar
   un error de conexión —el balanceador ya no existe.

## Práctica guiada: recrear el stack

### Lanzar el stack de nuevo

1. Con el stack eliminado, pulsar **Create stack → With new resources (standard)**.
2. Subir nuevamente el template `taller-semana1.yaml`. (Si la consola ofrece
   reutilizar el template anterior porque se subió recientemente, se puede hacer.)
3. En **Stack name**, usar exactamente el mismo nombre: `taller-<su-nombre>`.
4. En el campo del URI de la imagen, pegar el mismo URI de ECR usado antes.
    La imagen sigue en ECR — no es necesario volver a hacer el build.
5. Pulsar **Next**, aceptar las capacidades de IAM, y pulsar **Submit**.
6. En la pestaña **Events**, esperar a que el estado vuelva a **CREATE_COMPLETE**.

### Verificar que la aplicación está de nuevo en línea

1. En la pestaña **Outputs**, la URL del ALB puede ser diferente a la anterior —los
    balanceadores de carga generan nombres DNS únicos. Copiar el nuevo valor.
2. Abrir la URL en el navegador. La aplicación debe responder exactamente igual que
    antes. El ciclo completo está cerrado.

---

{#ejercicio-8}
### Ejercicio 8 — Destruir, y recrear, el ambiente

Eliminar el stack de CloudFormation por completo. Confirmar que la aplicación ya no
responde. Luego recrear el stack con los mismos parámetros y confirmar que la aplicación
vuelve a estar en línea.

::: solucion
**Destrucción:**

1. En la consola de AWS, abrir [**CloudFormation**](https://console.aws.amazon.com/cloudformation/home).
2. Seleccionar el stack `taller-<su-nombre>`.
3. Pulsar **Delete → Delete stack**.
4. En la pestaña **Events**, seguir los eventos hasta que el stack desaparezca de la
   lista.
5. Intentar abrir la URL del ALB anterior. El navegador debe mostrar un error de
   conexión, confirmando que el balanceador ya no existe.

**Recreación:**

1. Pulsar **Create stack → With new resources (standard)**.
2. Subir el template `taller-semana1.yaml` (o reutilizar el cargado anteriormente).
3. En **Stack name**, escribir `taller-<su-nombre>`.
4. En el campo del URI de la imagen, pegar el URI de ECR con la etiqueta `latest`.
   La imagen sigue disponible en ECR sin necesidad de un nuevo build.
5. Avanzar por las pantallas, aceptar las capacidades de IAM, y pulsar **Submit**.
6. En la pestaña **Events**, esperar a **CREATE_COMPLETE**.
7. En la pestaña **Outputs**, copiar la nueva URL del ALB.
8. Abrirla en el navegador. La guía del taller debe cargarse de nuevo —el ambiente
    está completamente restaurado.
:::

:::slide light
{{ejercicio-8}}
:::
