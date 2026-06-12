+++
title = "Destruir, y recrear, el ambiente"
+++

## Por qué se practica la destrucción desde el primer día

En un sistema operado manualmente, un error puede dejar el ambiente en un estado
inconsistente difícil de diagnosticar y corregir. El tiempo de recuperación depende de
cuánto se recuerde de cómo se construyó originalmente, y de si esa memoria es precisa.

Cuando el ambiente se define como código —en este caso, una plantilla de CloudFormation—
la situación cambia radicalmente. Si algo sale mal, la corrección no es reconstruir desde
la memoria: es borrar y volver a crear. El proceso es el mismo que siguió hace unos
minutos, tarda lo mismo, y produce exactamente el mismo resultado. El **costo de un
error se convierte en minutos de espera**, no en horas de diagnóstico.

Esta es la razón por la que se practica el ciclo completo de destrucción y recreación en
la Semana 1: para que en las semanas siguientes, si algo falla, la respuesta refleja no
sea la urgencia sino la calma. Se borra, se recrea, se sigue.

## Qué sobrevive a la destrucción del stack

Antes de borrar el stack, es útil entender qué destruye CloudFormation y qué no.

La plantilla `taller-semana1.yaml` crea y gestiona: el clúster ECS, el servicio Fargate,
el ALB, la tabla de DynamoDB, los roles de IAM, y la configuración de red. Todos esos
recursos **se eliminan** cuando se borra el stack.

Lo que **no** forma parte del stack y por lo tanto **sobrevive**:

- Su **repositorio de CodeCommit** con todo el historial de commits.
- Su **repositorio de ECR** con la imagen Docker publicada.
- El **proyecto de CodeBuild** configurado en la sección anterior.

Esto significa que al recrear el stack basta con volver a proporcionar el URI de la
imagen en ECR: el ambiente completo se reconstituye en minutos, sin volver a hacer el
build ni resubir el código.

## Práctica guiada: borrar el stack

### Iniciar la eliminación

1. En la consola de AWS, abra **CloudFormation**.
2. En la lista de stacks, seleccione su stack `taller-<su-nombre>`.
3. Pulse **Delete**.
4. En el diálogo de confirmación, pulse **Delete stack**.

### Seguir los eventos de borrado

1. CloudFormation comienza a eliminar los recursos en orden inverso al de creación
   (primero los que dependen de otros, luego los recursos base). La pestaña **Events**
   muestra cada eliminación en tiempo real.
2. Espere hasta que el stack desaparezca de la lista o, si la consola lo muestra,
   hasta que el estado sea **DELETE_COMPLETE**. El proceso toma entre 3 y 6 minutos.

> **Nota:** si algún recurso no puede eliminarse automáticamente (por ejemplo, una
> tabla de DynamoDB con protección contra eliminación, o un bucket de S3 con objetos),
> el estado cambiará a **DELETE_FAILED** y el evento fallido indicará el recurso y el
> motivo. El instructor le indicará cómo proceder en ese caso.

### Confirmar que la aplicación ya no está en línea

1. Intente abrir de nuevo la URL del ALB que usó antes. El navegador debería mostrar
   un error de conexión —el balanceador ya no existe.

## Práctica guiada: recrear el stack

### Lanzar el stack de nuevo

1. Con el stack eliminado, pulse **Create stack → With new resources (standard)**.
2. Suba nuevamente la plantilla `taller-semana1.yaml`. (Si la consola le ofrece
   reutilizar la plantilla anterior porque la subió recientemente, puede hacerlo.)
3. En **Stack name**, use exactamente el mismo nombre: `taller-<su-nombre>`.
4. En el campo del URI de la imagen, pegue el mismo URI de ECR que usó antes.
    La imagen sigue en ECR — no necesita volver a hacer el build.
5. Pulse **Next**, acepte las capacidades de IAM, y pulse **Submit**.
6. En la pestaña **Events**, espere a que el estado vuelva a **CREATE_COMPLETE**.

### Verificar que la aplicación está de nuevo en línea

1. En la pestaña **Outputs**, la URL del ALB puede ser diferente a la anterior —los
    balanceadores de carga generan nombres DNS únicos. Copie el nuevo valor.
2. Abra la URL en el navegador. La aplicación debe responder exactamente igual que
    antes. El ciclo completo está cerrado.

---

### Ejercicio 6 — Destruya, y recree, su ambiente

Elimine su stack de CloudFormation por completo. Confirme que la aplicación ya no
responde. Luego recree el stack con los mismos parámetros y confirme que la aplicación
vuelve a estar en línea.

::: solucion
**Destrucción:**

1. En la consola de AWS, abra **CloudFormation**.
2. Seleccione su stack `taller-<su-nombre>`.
3. Pulse **Delete → Delete stack**.
4. En la pestaña **Events**, siga los eventos hasta que el stack desaparezca de la
   lista.
5. Intente abrir la URL del ALB anterior. El navegador debe mostrar un error de
   conexión, confirmando que el balanceador ya no existe.

**Recreación:**

1. Pulse **Create stack → With new resources (standard)**.
2. Suba la plantilla `taller-semana1.yaml` (o reutilice la cargada anteriormente).
3. En **Stack name**, escriba `taller-<su-nombre>`.
4. En el campo del URI de la imagen, pegue el URI de ECR con la etiqueta `latest`.
   La imagen sigue disponible en ECR sin necesidad de un nuevo build.
5. Avance por las pantallas, acepte las capacidades de IAM, y pulse **Submit**.
6. En la pestaña **Events**, espere a **CREATE_COMPLETE**.
7. En la pestaña **Outputs**, copie la nueva URL del ALB.
8. Ábrala en el navegador. La guía del taller debe cargarse de nuevo —el ambiente
    está completamente restaurado.
:::
