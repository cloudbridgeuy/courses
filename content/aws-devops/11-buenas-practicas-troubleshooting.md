+++
title = "Buenas prácticas y troubleshooting"
+++

## Escribir templates que se puedan mantener

Un template funciona o no funciona, pero entre dos templates que funcionan hay una gran
diferencia de calidad: uno se entiende y se modifica con confianza, el otro es un campo
minado. Estas son las prácticas que separan a uno del otro.

### Nombres lógicos descriptivos

El nombre lógico de un recurso es para las personas que leen el template. `TablaApp` o
`ServicioApp` dicen qué es el recurso; `Resource1` o `MyTable2` no dicen nada. Como el
nombre lógico es además cómo el resto del template se refiere al recurso con `!Ref`, un
buen nombre hace legible cada conexión.

### Dejar que AWS genere los nombres físicos

Salvo que tenga una razón concreta, no fije el nombre físico de un recurso (`TableName`,
`RoleName`, etc.). Si lo deja sin especificar, CloudFormation genera uno único. Esto
evita un problema común: lanzar el mismo template dos veces y que falle porque el nombre
físico ya existe.

### Parámetros con valores por defecto

Un parámetro con `Default` documenta el valor habitual y permite lanzar el stack sin
tener que completarlo cada vez. Reserve los parámetros sin default para lo que de verdad
cambia entre lanzamientos —como el `ImageUri` de su template.

```yaml
Parameters:
  DesiredCount:
    Type: Number
    Default: 1
    Description: Número de tareas en ejecución.
```

### `DeletionPolicy` para lo que no debe perderse

Por defecto, borrar un stack borra todos sus recursos. Para los que guardan datos —una
tabla, un bucket— eso puede ser un desastre. `DeletionPolicy: Retain` le indica a
CloudFormation conservar el recurso aunque se borre el stack.

```yaml
  TablaApp:
    Type: AWS::DynamoDB::Table
    DeletionPolicy: Retain
```

::: warning
Nunca modifique a mano, desde la consola, un recurso que gestiona un stack. El template
y la realidad dejan de coincidir —eso es *drift*— y la próxima actualización del stack
puede revertir su cambio sin avisar. Si un recurso lo gestiona un stack, cámbielo solo a
través del stack.
:::

### Los outputs son un contrato

La sección `Outputs` es la interfaz pública del stack: lo que expone para que otros lo
usen. La URL del ALB es un output porque alguien —usted, otro stack, un script— necesita
ese valor sin tener que entrar a buscarlo. Trate los outputs como un contrato: nómbrelos
con claridad y exponga lo que realmente se consume desde afuera.

:::inline-slide light
## Buenas prácticas, en una línea

- Nombres lógicos **descriptivos**.
- Deje que AWS **genere los nombres físicos**.
- Parámetros con **`Default`** para lo habitual.
- **`DeletionPolicy: Retain`** para los datos.
- Nunca edite a mano un recurso gestionado por un stack.
:::

## Troubleshooting: leer un fallo

Tarde o temprano un stack falla. Saber leer el fallo es lo que convierte media hora de
frustración en dos minutos de diagnóstico.

### El primer evento fallido es la causa

Cuando un stack entra en `ROLLBACK_IN_PROGRESS`, la pestaña **Events** se llena de
mensajes: el recurso que falló, y luego todos los que se deshacen en el rollback. El
ruido del rollback puede esconder la causa real.

La técnica: ordene los eventos por tiempo y busque el **primer** evento con estado
`CREATE_FAILED` (o `UPDATE_FAILED`). Ese, y no los posteriores, contiene el motivo real
—el resto son consecuencias. La columna de razón (*status reason*) suele decir
exactamente qué pasó.

### Fallos comunes y qué los causa

| Síntoma | Causa habitual |
| --- | --- |
| `requires capabilities: [CAPABILITY_IAM]` | El template crea roles de IAM; falta marcar la casilla de capacidades al lanzar. |
| `already exists` | Colisión de nombre físico: el recurso ya existe (a menudo, por fijar un nombre a mano). |
| `is not authorized to perform` | Permisos insuficientes en el usuario o rol que lanza el stack. |
| `limit exceeded` | Se alcanzó un límite de la cuenta (por ejemplo, número de VPC o de EIP). |

La primera fila explica la casilla de capacidades que marcó en la Semana 1: no era un
trámite, era CloudFormation pidiendo permiso explícito para crear roles de IAM.

:::slide
## Leer un fallo

1. Abra la pestaña **Events**.
2. Busque el **primer** evento `..._FAILED`.
3. Lea su *status reason* — esa es la causa.

El resto de los eventos del rollback son consecuencias.
:::

::: extra Validar el template antes de lanzarlo
La herramienta de línea de comandos **`cfn-lint`** revisa un template en busca de errores
de sintaxis, propiedades inválidas, y referencias rotas, sin necesidad de lanzar nada.
Integrada en el editor o en el pipeline, atrapa la mayoría de los errores antes de llegar
a la consola. La consola también ofrece un botón **Validate** que hace una verificación
básica de sintaxis al subir el template.
:::
