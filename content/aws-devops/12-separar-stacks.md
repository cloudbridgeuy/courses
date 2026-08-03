+++
title = "Separar los stacks por ciclo de vida"
+++

## Un stack, tres ciclos de vida
:::inline-slide with-title light

El stack monolítico de la Semana 1 cumplió su función: un archivo, algunos parámetros, un
ambiente completo. Pero dentro de ese stack conviven recursos que envejecen a ritmos
muy distintos:

:::skip
- La **red** (VPC, subredes, grupos de seguridad) casi nunca cambia. Se define una vez
  y varias aplicaciones podrían compartirla.
- Los **datos** (la tabla de DynamoDB) deben sobrevivir. Destruir el ambiente no
  debería tocarlos.
- La **aplicación** (servicio, task definition, balanceador) cambia todo el tiempo, y
  es deliberadamente descartable —el seguro del taller depende de eso.
:::

:::add visibility=slide
Tenemos 3 o 4 cortes claros en nuestro stack actual: `red`, `datos`, `aplicación`, y `clúster`.
:::

::: warning
Mientras los tres viven en un solo stack, comparten un solo destino: no se puede borrar
la aplicación sin borrar la tabla, ni recrear el ambiente sin recrear la red. La
[guía oficial de buenas prácticas de CloudFormation](https://docs.aws.amazon.com/AWSCloudFormation/latest/UserGuide/best-practices.html)
recomienda exactamente esta separación: **organizar los stacks por ciclo de vida y
por responsable**, no por conveniencia de tener todo junto.
::: # warning
::: # inline-slide

## Cómo encontrar el corte

Decir "tres stacks" es fácil cuando alguien ya los dividió. La pregunta útil es la
otra: frente a un template propio de cuatrocientas líneas, ¿cómo se decide **dónde**
cortar? El procedimiento es mecánico, y tiene cuatro pasos.

### Paso 1: listar los recursos

El template de la Semana 1 tiene veintiún recursos. Escritos como lista plana, sin la
estructura del archivo, dejan de esconderse entre las propiedades:

`TablaApp` · `ClusterApp` · `ServicioApp` · `TareaApp` · `GrupoLogs` ·
`RolEjecucion` · `RolTarea` · `VpcApp` · `SubredPublicaA` · `SubredPublicaB` ·
`GatewayInternet` · `VinculoGateway` · `TablaRutas` · `RutaSalida` ·
`AsociacionSubredA` · `AsociacionSubredB` · `GrupoSeguridadALB` ·
`GrupoSeguridadServicio` · `BalanceadorApp` · `GrupoDestino` · `ListenerHTTP`

### Paso 2: agrupar por ritmo de cambio

A cada recurso se le hace una sola pregunta: **¿cada cuánto cambia?** No qué servicio
es, ni a qué capa de la arquitectura pertenece —cada cuánto cambia—. Los que responden
lo mismo van juntos.

| Grupo | Ritmo | Recursos |
| --- | --- | --- |
| **Red** | Casi nunca. Una vez por ambiente. | Los 11 de EC2: VPC, subredes, gateway, rutas, y los dos grupos de seguridad |
| **Datos** | Nunca se recrea. Debe sobrevivir. | `TablaApp` |
| **Aplicación** | En cada despliegue. | Los 9 restantes: ECS, ALB, logs, y los roles |

Los tres grupos salen solos, y ninguno depende de gustos: la VPC no cambia porque se
despliegue una versión nueva, y la tabla no se puede recrear sin perder lo que guarda.

### Paso 3: marcar las referencias que cruzan

Este es el paso que convierte una intuición en un diseño. Se recorren los `!Ref` y
`!GetAtt` del template, y se marcan **solo los que ahora cruzan de un grupo a otro**.
Los que quedan dentro de un mismo grupo no importan: siguen siendo referencias
internas.

En el template de la Semana 1 cruzan siete, y ni una más:

| Referencia que cruza | De | Hacia |
| --- | --- | --- |
| El ID de la VPC (lo usa `GrupoDestino`) | Aplicación | Red |
| El ID de la subred A (la usan el servicio y el ALB) | Aplicación | Red |
| El ID de la subred B (la usan el servicio y el ALB) | Aplicación | Red |
| El ID del grupo de seguridad del ALB | Aplicación | Red |
| El ID del grupo de seguridad del servicio | Aplicación | Red |
| El nombre de la tabla (variable de entorno del contenedor) | Aplicación | Datos |
| El ARN de la tabla (política de `RolTarea`) | Aplicación | Datos |

### Paso 4: cada cruce es un export

La lista del paso 3 **es** el contrato. Cada fila se vuelve un `Export` en el stack de
origen, y un `Fn::ImportValue` en el de destino. Nada más se exporta: lo que no cruza,
no se publica.

Y aquí se puede verificar el método contra la realidad. El stack de red que provee el
instructor tiene exactamente **cinco** exports, y el de datos exactamente **dos**. Son
las siete filas de la tabla, una por una. Los templates no salieron de la intuición de
nadie: salieron de este procedimiento.

::: info
Que la flecha vaya siempre de la aplicación hacia los otros dos no es casualidad. Las
dependencias apuntan de lo que cambia seguido hacia lo que cambia poco, nunca al revés.
Un export del stack de aplicación hacia el de red sería una señal de que el corte está
mal puesto.
:::

:::slide
## Cuatro pasos para dividir un stack

1. **Listar** los recursos.
2. **Agrupar** por ritmo de cambio.
3. **Marcar** las referencias que cruzan un grupo.
4. Cada cruce es un **export**.
:::

## Eje de implementación

El ciclo de vida alcanza para este taller, y suele alcanzar. Cuando no alcanza, hay
otros dos criterios, y la guía de AWS nombra el primero junto al ciclo de vida.

**Por responsable.** Si dos grupos de recursos los aprueba gente distinta, van en
stacks distintos, aunque cambien al mismo ritmo. La red la administra el equipo de
plataforma, la aplicación su equipo de desarrollo. Separarlos permite que cada uno
despliegue lo suyo sin pedir permiso ni tocar lo ajeno. Un stack compartido convierte
cada cambio en una negociación.

**Por radio de impacto.** ¿Qué se lleva puesto un error? Un stack grande falla entero
y hace rollback entero, así que un error en una etiqueta puede revertir un despliegue
completo. Aislar lo delicado —los datos— limita el daño de un error en lo que no lo es.

En la práctica los tres ejes suelen coincidir: lo que cambia a otro ritmo, lo aprueba
otra gente, y rompe otras cosas. Cuando no coinciden, **el ciclo de vida decide**,
porque es el único de los tres que la plataforma hace cumplir.

:::slide
## Tres ejes para cortar

| Eje | La pregunta |
| --- | --- |
| **Ciclo de vida** | ¿Cada cuánto cambia? |
| **Responsable** | ¿Quién lo aprueba? |
| **Radio de impacto** | ¿Qué se lleva puesto si falla? |

Cuando no coinciden, manda el **ciclo de vida**.
:::

:::inline-slide light
## Dónde va la seguridad

Red, datos, y aplicación son capas fáciles de ubicar. La seguridad no: un rol de IAM no
"pertenece" de manera obvia a ninguna, porque siempre conecta dos cosas. `RolTarea` es
el ejemplo exacto. Lo usa la aplicación, y da acceso a la tabla, que vive en otro
stack.

:::skip
El template de aplicación resuelve el caso así:
:::

```yaml
  RolTarea:
    Type: AWS::IAM::Role                     # vive en el stack de aplicación
    Properties:
      Policies:
        - PolicyName: acceso-tabla-app
          PolicyDocument:
            Statement:
              - Action: [dynamodb:GetItem, dynamodb:PutItem]
                Resource:
                  Fn::ImportValue: !Sub "${DatosStackName}-tabla-arn"
```

:::skip
El rol vive con la **aplicación**, no con los datos, y de ahí sale la regla general:
:::

::: info
**Un permiso sigue a quien lo consume, no al recurso que protege.**
:::
:::

La razón es la de siempre, el ciclo de vida. El permiso cambia cuando cambia la
aplicación (una función nueva necesita una acción nueva), no cuando cambia la tabla.
Ponerlo en el stack de datos obligaría a tocar los datos para cambiar el código, que es
justo lo que la separación quiere evitar. Y la dependencia queda apuntando en la
dirección correcta: la aplicación importa el ARN, el stack de datos no sabe quién lo
usa.

:::inline-slide
### Cuándo sí conviene un stack de seguridad aparte

La regla anterior no dice "nunca separar la seguridad". Dice que un rol de un solo
consumidor va con su consumidor. Un stack de seguridad propio se justifica cuando
aparece cualquiera de estas tres cosas:

:::skip
- **Roles compartidos por varias aplicaciones.** Si tres stacks de aplicación usan el
  mismo rol, ya no sigue a un consumidor: es infraestructura común, como la red.
- **Un aprobador distinto.** Cuando el equipo de seguridad revisa los cambios de IAM y
  nadie más los toca, es el eje "por responsable" pidiendo un corte.
- **Recursos de gobierno**: *permission boundaries*, políticas de contraseñas,
  proveedores de identidad, roles de auditoría. No sirven a una aplicación; son la
  cuenta entera.
:::

:::add visibility=slide
1. **Roles compartidos por varias aplicaciones.**
2. **Un aprobador distinto.**
3. **Recursos de gobierno.** (contraseñas, IdP, roles de auditoría.
::: #add
::: #inline-slide

Ninguna de las tres se da en el taller: dos roles, un solo consumidor, un solo
responsable. Por eso los roles viven con la aplicación, y por eso no hay un stack de
seguridad. La decisión de **no** dividir también se toma con el mismo criterio.

## Corte por Aplicación
:::inline-slide light

:::skip
El procedimiento de arriba se aplicó a una pregunta concreta: **un** ambiente, **una**
aplicación. Cambiando la pregunta, cambia la respuesta. Supongamos que mañana hay que
poner una segunda aplicación en la misma cuenta (otra imagen, otro equipo, la misma
red). ¿Alcanza con lanzar el stack de aplicación una segunda vez?

Alcanza, y sale mal. Porque el grupo "Aplicación" del paso 2 escondía una diferencia
que con una sola aplicación no se nota. A cada uno de esos nueve recursos hay que
hacerle ahora una segunda pregunta: **¿cuántas aplicaciones lo usan?**
:::


| Recurso | ¿Cuántas aplicaciones lo usan? |
| --- | --- |
| `ClusterApp` | Todas. Es una agrupación lógica de servicios. |
| `BalanceadorApp` | Todas. Un solo punto de entrada, un solo nombre DNS. |
| `ListenerHTTP` | Todas. Escucha un puerto del balanceador compartido. |
| `ServicioApp`, `TareaApp` | Una. Es *esta* imagen, con *esta* configuración. |
| `GrupoDestino` | Una. Describe el puerto y el health check de *esta* aplicación. |
| `GrupoLogs` | Una. Los logs de *esta* aplicación. |
| `RolEjecucion`, `RolTarea` | Una. Los permisos de *esta* aplicación. |

:::

Los tres de arriba forman una capa que hasta ahora no tenía nombre: la
**plataforma**, el sustrato de ejecución sobre el que corre cualquier
aplicación.

:::inline-slide with-title
### El clúster engaña; el balanceador no

:::skip
Duplicar el clúster no duele, y ese es exactamente el problema. Con Fargate, un clúster
de ECS **no reserva capacidad**: es una agrupación lógica, y no se factura. Diez
clústeres vacíos cuestan lo mismo que uno. Por eso el error sobrevive sin que nadie lo
note, hasta que aparece un inventario con treinta clústeres de una aplicación cada uno,
y ya nadie sabe cuál es cuál.

El balanceador sí duele. Un ALB se factura por **hora de existencia**, además del
tráfico que procesa. Dos balanceadores para el mismo tráfico no reparten el costo: lo
duplican en su parte fija. Y el costo no es lo único: dos balanceadores son dos nombres
DNS, dos certificados, y dos lugares donde configurar lo mismo.
:::

:::add visibility=slide
Duplicar el clúster no duele, el balanceador sí.
:::

::: info
El clúster compartido tiene un límite que conviene saber: los exports de CloudFormation
viven en **una sola cuenta y una sola región**. Una plataforma compartida sirve a las
aplicaciones de esa cuenta. Entre cuentas, lo que se comparte es la **red**, con AWS
Resource Access Manager, que sí permite compartir subredes, y cada cuenta pone su
propio clúster. Como el clúster es gratis, esa duplicación no cuesta nada; el ALB, sí.
:::
::: #inline-slide

:::inline-slide with-title
### La regla del listener: el contrato al revés

:::skip
Sacar el balanceador del stack de aplicación plantea un problema nuevo. Hasta acá cada
cruce se resolvía **importando**: la aplicación lee un valor que otro stack publicó. El
balanceador necesita lo contrario. La aplicación no quiere *leer* el listener: quiere
**agregarle** algo, una entrada que diga "lo que venga por esta ruta, mandámelo a mí".
:::

:::add visibility=slide
La aplicación no necesita *leer* el `listener`: quiere *agregarle* algo.
:::

El recurso que hace eso es `AWS::ElasticLoadBalancingV2::ListenerRule`, y vive en el
stack de aplicación:

```yaml
  ReglaHTTP:
    Type: AWS::ElasticLoadBalancingV2::ListenerRule
    Properties:
      ListenerArn:
        Fn::ImportValue: !Sub "${PlataformaStackName}-listener-http-arn"
      Priority: !Ref Prioridad
      Conditions:
        - Field: path-pattern
          PathPatternConfig:
            Values: [!Ref RutaPath]
      Actions:
        - Type: forward
          TargetGroupArn: !Ref GrupoDestino
```
::: #inline-slide

Lo importante es lo que **no** pasa. El stack de plataforma no menciona ninguna
aplicación, y no cambia cuando aparece una nueva: su listener tiene como acción por
defecto un `fixed-response` con un 404, y todo el tráfico útil lo colocan las reglas.
La dependencia sigue apuntando en la dirección correcta, igual que con la red y los datos.

Esta forma ya apareció en el taller: el template opcional de HTTPS de la Semana 1 le
agrega un listener al ALB de otro stack sin modificarlo. Es el mismo movimiento.

:::inline-slide with-title light
### Prioridades: la más específica primero

Las reglas de un listener se evalúan **de menor a mayor `Priority`**, y gana la primera
que coincide. Si ninguna coincide, corre la acción por defecto. De ahí salen dos
consecuencias prácticas:

- La regla más **general** lleva el número más **alto**. Una aplicación que atiende
  `/*` con prioridad 1 se queda con todo el tráfico, y ninguna otra regla se llega a
  evaluar.
- La prioridad es **única por listener**. Dos stacks de aplicación con el mismo número
  no conviven: el segundo falla al crear la regla, con un mensaje claro.

::: info
El enrutado por ruta exige que la aplicación sepa servirse desde su prefijo: una app que
genera enlaces absolutos a `/estilos.css` se rompe detrás de `/*`. Cuando eso no se
puede cambiar, se enruta por **host** en vez de por ruta, `Field: host-header`, un
dominio por aplicación, un solo balanceador, y el problema desaparece.
:::
::: #inline-slide

:::inline-slide
## Cuándo no dividir

:::skip
Dividir tiene precio, y conviene decirlo antes de que parezca gratis. Tres stacks son
tres despliegues a coordinar, un orden de borrado obligatorio, y valores congelados
mientras alguien los importe. Un ambiente partido en ocho stacks para lo que necesitaba
dos no es más mantenible: es el mismo sistema, con más ceremonia.
:::

:::add visibility=slide
Dividir tiene precio y conviene decirlo. Hay señales que podemos usar para saber cuando
la división no esta sumando.

1. Dos stacks **siempre se depliegan juntos.**
2. Contratos enormes. En general, **un contrato sano es chico.**
3. Cambios **cruzan fronteras todo el tiempo.**
:::

::: info
Regla práctica: **empezar junto, y dividir cuando duela.**
:::
::: #inline-slide

Las señales de que un corte sobra:

- Los dos stacks **siempre se despliegan juntos**. Si nunca se actualiza uno sin el
  otro, no tienen ciclos de vida distintos: tienen uno solo, escrito dos veces.
- El contrato es **enorme**. Diez o quince exports entre dos stacks indican que el corte
  pasó por el medio de algo que era una sola pieza. Un contrato sano es chico.
- Los cambios cruzan la frontera **todo el tiempo**. Si cada tarea toca los dos stacks,
  la frontera está estorbando en vez de proteger.

La regla práctica: **empezar junto, y dividir cuando duela**. El dolor es concreto y se
reconoce (no poder borrar el ambiente sin perder los datos es exactamente el dolor que
motiva esta sección). Dividir antes de sentirlo es adivinar.

:::

:::inline-slide light
## La arquitectura en cuatro stacks

```mermaid
%%{init: {"flowchart": {"nodeSpacing": 60, "rankSpacing": 55}, "themeVariables": {"edgeLabelBackground": "#ffffff"}}}%%
flowchart TB
  APP["<b>Stack de aplicación</b><br/>servicio · tarea · IAM<br/><i>cambia seguido</i>"]
  PLAT["<b>Stack de plataforma</b><br/>clúster ECS · ALB · listener<br/><i>compartido por las apps</i>"]
  DATOS[("<b>Stack de datos</b><br/>TablaApp<br/><i>debe sobrevivir</i>")]
  RED["<b>Stack de red</b><br/>VPC · subredes · SGs<br/><i>casi nunca cambia</i>"]

  APP -->|"clúster · listeners"| PLAT
  APP -->|"nombre y ARN<br/>de la tabla"| DATOS
  APP -->|"subredes · SG"| RED
  PLAT -->|"subredes · SG del ALB"| RED

  classDef appNode fill:#fdf2f8,stroke:#e7157b,stroke-width:2px,color:#831843
  classDef platNode fill:#fef3c7,stroke:#d97706,color:#451a03
  classDef datosNode fill:#f0fdf4,stroke:#16a34a,color:#14532d
  classDef redNode fill:#f1f5f9,stroke:#475569,color:#0f172a
  class APP appNode
  class PLAT platNode
  class DATOS datosNode
  class RED redNode
```

Cada flecha es un `Fn::ImportValue`: el stack lee los exports del stack al que
apunta. Cuanto más abajo, más estable.
:::

:::inline-slide with-title light
En el directorio `/infra/templates` esta el mismo template de la semana 1 dividido en 4:

| Template | Stack | Contiene |
| --- | --- | --- |
| `taller-aws-devops-semana2-red.yaml` | `taller-aws-<su-nombre>-red` | VPC, subredes, gateway, grupos de seguridad |
| `taller-aws-devops-semana2-datos.yaml` | `taller-aws-<su-nombre>-datos` | La tabla de DynamoDB, con `DeletionPolicy: Retain` |
| `taller-aws-devops-semana2-datos-import.yaml` | `taller-aws-<su-nombre>-datos` | La misma tabla, sin `Outputs`: la versión que exige la operación de import |
| `taller-aws-devops-semana2-plataforma.yaml` | `taller-aws-<su-nombre>-plataforma` | Clúster de ECS, balanceador, listeners |
| `taller-aws-devops-semana2-app.yaml` | `taller-aws-<su-nombre>-app` | Servicio, task definition, target group, regla, logs, roles |


:::app
<cb-file path="./infra/templates/taller-aws-devops-semana2-app.yaml" type="yaml" toggleable full-path></cb-file>
:::

:::app
<cb-file path="./infra/templates/taller-aws-devops-semana2-plataforma.yaml" type="yaml" toggleable full-path></cb-file>
:::

:::app
<cb-file path="./infra/templates/taller-aws-devops-semana2-datos.yaml" type="yaml" toggleable full-path></cb-file>
:::

:::app
<cb-file path="./infra/templates/taller-aws-devops-semana2-datos-import.yaml" type="yaml" toggleable full-path></cb-file>
:::

:::app
<cb-file path="./infra/templates/taller-aws-devops-semana2-red.yaml" type="yaml" toggleable full-path></cb-file>
:::

::: info
Para el diagrama de red, hay una versión con VPC existente.
:::

:::app
<cb-file path="./infra/templates/taller-aws-devops-semana2-red-existente.yaml" type="yaml" toggleable full-path></cb-file>
:::
::: #inline-slide

Nótese la única flecha nueva que no sale de la aplicación: **plataforma → red**. El
balanceador necesita las subredes y el grupo de seguridad, así que importa del stack de
red igual que lo hacía antes. Ninguna flecha apunta hacia la aplicación, y esa es la
comprobación de que el corte quedó bien puesto.

## El contrato entre stacks

Separar los stacks obliga a hacer explícito lo que antes era un `!Ref` interno. El
stack de red **exporta** sus valores con nombre:

```yaml
Outputs:
  VpcId:
    Value: !Ref VpcApp
    Export:
      Name: !Sub "${AWS::StackName}-vpc-id"
```

Y el stack de aplicación los **importa** por ese nombre:

```yaml
      VpcId:
        Fn::ImportValue: !Sub "${RedStackName}-vpc-id"
```

Esto es la versión entre stacks de los outputs como contrato que se vio en la sección
anterior. Y como todo contrato, obliga: mientras el stack de aplicación importe un
valor, CloudFormation **impide borrar o modificar** el stack que lo exporta. El orden
de borrado deja de ser una convención y pasa a estar garantizado por CloudFormation:
primero la aplicación, después la plataforma y los datos, al final la red.

:::inline-slide
### El precio del contrato

:::skip
Esa garantía tiene un costo que conviene conocer antes de apoyarse en exports para
todo. Mientras alguien importa un export, su valor **no se puede cambiar**: no solo
está protegido el stack, está congelado el valor. Cambiar el CIDR de la VPC del stack
de red, con la aplicación importándolo, exige borrar primero el stack de aplicación,
cambiar la red, y volver a crearla. La rigidez es el precio de la seguridad.
:::

:::add visibility=slide
Mientras alguien importa un `export`, su valor **no se puede cambiar**.
:::

:::skip
Los otros dos límites son de alcance. Un export vive en **una sola región y una sola
cuenta**: no hay import entre regiones ni entre cuentas. Y el nombre del export es
único en la región, de ahí el prefijo `${AWS::StackName}` que convive con el resto de
los participantes.
:::

:::add visibility=slide
Un `export` vive en `**una sola región y una sola cuenta.**
:::

::: info
La regla práctica que sale de esto: exportar lo que de verdad es un contrato estable
(identificadores de red, ARNs de recursos compartidos) y no exportar valores que se
espera ajustar seguido.
:::
::: #inline-slide

::: extra El contrato en acción: reutilizar una VPC existente
La separación paga un dividendo inmediato. En cuentas donde no se crea una VPC por
participante —por cuota o por política del cliente—, el instructor provee la variante
`taller-aws-devops-semana2-red-existente.yaml`: recibe como parámetros una **VPC
existente** (la VPC por defecto de la cuenta sirve) y **dos subredes públicas en
zonas de disponibilidad distintas**, crea únicamente los grupos de seguridad, y
publica todo bajo los **mismos cinco exports** que el stack de red estándar.

Los stacks de datos, plataforma, y aplicación no cambian ni una línea. Ese es el
punto: quien importa `-vpc-id` no puede distinguir —ni necesita distinguir— si la
red la creó CloudFormation o la adoptó de la cuenta. Un contrato bien definido hace
intercambiable a quien lo cumple.

Si la VPC disponible no tiene subredes públicas utilizables, el template
`taller-aws-devops-extra-subredes-publicas.yaml` —desplegado una sola vez por quien
administra la cuenta— las crea: dos subredes en zonas distintas, su tabla de rutas,
y la salida a internet, con el Internet Gateway existente como parámetro opcional
(si no se indica, lo crea y lo adjunta).
:::

:::inline-slide
## El problema: la tabla ya tiene datos

:::skip
Recrear la red y la aplicación es gratis. La tabla es distinta: los contadores
que la guía fue acumulando viven ahí. Borrar el stack monolítico y lanzar los
tres nuevos destruiría esos datos.

CloudFormation tiene una respuesta específica para esto: **importar recursos**. Un
stack puede *adoptar* un recurso que ya existe: sin tocarlo, sin recrearlo. Siempre
que el template lo describa tal como es y declare una `DeletionPolicy` explícita. La
migración completa usa tres piezas que ya se conocen, más el import:
:::

:::add visibility=slide
¿Como migramos la tabla sin eliminarla?

Utilizamos un `import`, para *adoptar* un recurso a otro template.
:::

1. `DeletionPolicy: Retain` sobre la tabla, aplicado con un change set.
2. Borrar el stack monolítico —todo muere, salvo la tabla, que queda **huérfana**:
   viva, funcionando, pero sin stack que la gestione.
3. Crear los stacks de red, plataforma, y aplicación como siempre.
4. Crear el stack de datos **importando** la tabla huérfana.
:::

:::slide
## La migración

1. `DeletionPolicy: Retain` en la tabla → change set.
2. Borrar el stack monolítico → la tabla queda **huérfana**.
3. Crear el stack de red, y encima el de plataforma.
4. Crear el stack de datos **importando** la tabla.
5. Crear el stack de aplicación.

La tabla nunca se recrea. Los datos nunca se mueven.

:::app
<cb-goto path="Práctica guiada: la migración"></cb-goto>
::: # add
:::

## Práctica guiada: la migración

### Proteger la tabla con `Retain`

1. Abrir su template de CloudFormation en el editor y agregar la política sobre la tabla:

   ```yaml
   TablaApp:
     Type: AWS::DynamoDB::Table
     DeletionPolicy: Retain
   ```
2. Ir a la configuración de variables de entorno de su aplicación y agregar la siguiente variable:

   ```yaml
            - Name: CB_APPS_SECRET
              Value: secreto
   ```

2. En [**CloudFormation**](https://console.aws.amazon.com/cloudformation/home),
   seleccionar el stack `taller-aws-<su-nombre>` y aplicar el cambio con un change set,
   como en la sección anterior: **Stack actions → Create change set for current
   stack**, subir el template modificado, y ejecutarlo. `TablaApp` aparece como
   **Modify** sin reemplazo —la política es metadata del stack, no toca la tabla.

### Dejar una marca en la tabla

Antes de migrar, escribir un dato que demuestre, al final, que nada se perdió.

1. Resolver el nombre físico de la tabla a partir del stack verificando los `Outputs` del stack.

   Con la `awscli`:

   ```bash
   export TALLER=taller-aws-<su-nombre>
   TABLA=$(aws cloudformation describe-stack-resources \
     --stack-name "$TALLER" \
     --logical-resource-id TablaApp \
     --query "StackResources[0].PhysicalResourceId" \
     --output text)
   echo "$TABLA"
   ```

2. Incrementar un contador de prueba con la app `counter` de la guía. El botón
   emite un evento al servidor que sirve la página, y este ejecuta sobre la
   tabla el mismo `UpdateItem` con `ADD` que se lanzaría a mano con la CLI.
   Como escribe en la tabla del ambiente que sirve la página, usar esta misma
   sección en la guía del propio stack. Encontrar la **UrlBase** de
   `taller-aws-<su-nombre>` en los `Outputs`, desbloquear la app con el
   secreto, y pulsar el botón un par de veces:

:::app
<cb-counter key="migracion" label="Contador de migración"></cb-counter>
:::

3. Verificar, refrescando la página, que el valor persiste.

### Borrar el stack monolítico

1. Con el change set aplicado, pulsar **Delete → Delete stack**. Es el mismo
   teardown de la Semana 1, con una diferencia: en la pestaña **Events**, la tabla
   aparece como **DELETE_SKIPPED**. CloudFormation la deja atrás, intacta.

::: info
Si está ejecutando al `app` en modo HTTPS, debe eliminar el `Stack` que lo configura
primero.
:::

2. Confirmar que la tabla sigue viva, ahora sin stack desde el servicio DynamoDB o
   utilizando la `awscli`.

   ```bash
   aws dynamodb describe-table --table-name "$TABLA" \
     --query "Table.TableStatus" --output text
   ```

   Debe responder `ACTIVE`.

### Crear el stack de red

1. Pulsar **Create stack → With new resources (standard)**.
2. Subir `taller-aws-devops-semana2-red.yaml`.
3. En **Stack name**, escribir `taller-aws-<su-nombre>-red`. No tiene parámetros.
4. Pulsar **Next** hasta **Submit**, y esperar a **CREATE_COMPLETE**.

### Crear el stack de plataforma

1. Pulsar **Create stack → With new resources (standard)**.
2. Subir `taller-aws-devops-semana2-plataforma.yaml`.
3. En **Stack name**, escribir `taller-aws-<su-nombre>-plataforma`.
4. En **RedStackName**, escribir `taller-aws-<su-nombre>-red`. Dejar **NombreDominio**
   y **HostedZoneId** vacíos: son la parte opcional de HTTPS, más abajo.
5. Pulsar **Next** hasta **Submit**, y esperar a **CREATE_COMPLETE**.
6. Abrir la **UrlBase** de la pestaña **Outputs**. Responde `404` con el texto
   *Ninguna aplicacion responde en esta ruta.* —es correcto: el balanceador existe y
   todavía no hay ninguna regla que lo enrute a ningún lado.

### Crear el stack de datos, importando la tabla

Este stack no se crea: se crea *alrededor* de la tabla que ya existe. Y son dos
pasos, porque la operación de import solo acepta los recursos que se importan: un
template con `Outputs` se rechaza. Primero el import, con la versión sin `Outputs`;
después un update, con el template completo, para publicar los exports.

1. Pulsar **Create stack → With existing resources (import resources)**.
2. Subir `taller-aws-devops-semana2-datos-import.yaml`.
3. En la pantalla **Identify resources**, CloudFormation lista los recursos del
   template que necesitan un identificador. Para `TablaApp`, pegar en **TableName**
   el nombre físico de la tabla (el valor de `$TABLA`).
4. En **Stack name**, escribir `taller-aws-<su-nombre>-datos`, y pulsar **Next**.
5. Revisar el resumen: la operación es **Import**, y no crea ni modifica nada más.
   Pulsar **Import resources**.
6. En la pestaña **Events**, esperar a **IMPORT_COMPLETE**. La tabla no se reinició
   ni se recreó: solo cambió quién la gestiona.
7. Con el stack ya creado, aplicar el template completo (el mismo, más los
   `Outputs`) con un update **directo** haciendo click en **Update stack ->
   Make a direct update**. Y seleccionando **Replace existing template**. Use
   el template `taller-aws-devops-semana2-datos.yaml`.

   Desde la terminal, parado en el directorio donde está descargado el template:

   ```bash
   aws cloudformation update-stack \
     --stack-name taller-aws-<su-nombre>-datos \
     --template-body file://taller-aws-devops-semana2-datos.yaml
   aws cloudformation wait stack-update-complete \
     --stack-name taller-aws-<su-nombre>-datos
   ```

   El cambio debe ser directo, y no con un change set, por una limitación de
   CloudFormation: un change set que solo agrega o modifica `Outputs` se
   reporta como *The submitted information didn't contain changes* y no se
   puede ejecutar. El update directo sí los aplica, y no toca la tabla: en la
   pestaña **Outputs** del stack aparecen los dos exports que el stack de
   aplicación va a importar.

### Crear el stack de aplicación

1. Pulsar **Create stack → With new resources (standard)**.
2. Subir `taller-aws-devops-semana2-app.yaml`.
3. En **Stack name**, escribir `taller-aws-<su-nombre>-app`.
4. Completar el **URI de la imagen** en ECR, y los nombres de los otros tres stacks:
   `taller-aws-<su-nombre>-red`, `taller-aws-<su-nombre>-datos`, y
   `taller-aws-<su-nombre>-plataforma`. Dejar **RutaPath**, **Prioridad**, y
   **UsarHttps** en sus valores por defecto: esta es la única aplicación, y atiende
   todo el tráfico.
5. Aceptar la capacidad de IAM, pulsar **Submit**, y esperar a **CREATE_COMPLETE**.

### Verificar que nada se perdió

1. Volver a abrir la **UrlBase** del stack de plataforma —la misma URL que antes
   devolvía `404`—. Ahora carga la guía: lo único que cambió es que existe una regla
   que enruta esa ruta al nuevo servicio.
2. Releer el contador escrito antes de la migración. En la guía recién
   desplegada, el visor muestra el valor que se escribió antes de borrar el stack:

:::app
<cb-counter mode="view" key="migracion" label="Contador de migración"></cb-counter>
:::

   El valor sigue ahí. El ambiente se desarmó y se rearmó en cuatro stacks, y los
   datos nunca dejaron de existir.

## Opcional: HTTPS sobre el balanceador compartido

El certificado y el dominio pertenecen al balanceador, y el balanceador ahora es de la
plataforma. Por eso el stack de plataforma acepta dos parámetros opcionales,
`NombreDominio` y `HostedZoneId`, y con ellos agrega cinco recursos que sin ellos no
existen: el certificado de ACM, el listener 443, el registro alias en Route 53, la
regla de entrada del puerto 443, y el export del listener HTTPS.

Es la sección `Conditions` en su uso más típico:

```yaml
Conditions:
  ConHttps: !And
    - !Not [!Equals [!Ref NombreDominio, ""]]
    - !Not [!Equals [!Ref HostedZoneId, ""]]

Resources:
  ListenerHTTPS:
    Condition: ConHttps          # sin dominio, este recurso no se crea
    Type: AWS::ElasticLoadBalancingV2::Listener
```

Para activarlo sobre un ambiente ya desplegado:

1. Actualizar el stack `taller-aws-<su-nombre>-plataforma` con un change set,
   completando los dos parámetros. El change set muestra cinco **Add** y ningún
   reemplazo.
2. Actualizar el stack de aplicación poniendo **UsarHttps** en `si`. Eso agrega su
   regla también al listener 443, con la misma ruta y la misma prioridad.

::: info
Un certificado de ACM validado por DNS bloquea la creación del stack hasta que se
emite. CloudFormation escribe el registro de validación en la hosted zone y espera. Si
el ID de la zona es incorrecto, el stack queda en `CREATE_IN_PROGRESS` hasta agotar el
tiempo: conviene verificarlo antes.
:::

:::inline-slide
## Agregar una segunda aplicación

Con el clúster y el balanceador fuera del stack de aplicación, agregar una segunda
aplicación deja de ser un problema de diseño y pasa a ser un formulario. **No hay nada
que escribir**: es el mismo archivo, con otros parámetros.

:::skip
La segunda aplicación del taller es un **servidor de eco**: contesta cada pedido con un
JSON que describe ese mismo pedido —método, ruta, query, headers, cuerpo, de dónde vino,
y por qué red pasó—. No hace falta construir ni publicar otra imagen, porque ya viene
adentro de la que se usó toda la semana: es un subcomando del mismo binario.
:::

:::app
<cb-goto path="Desplegar otra aplicación en el mismo cluster"></cb-goto>
:::
::: #inline-slide

### Desplegar otra aplicación en el mismo cluster

1. Pulsar **Create stack → With new resources (standard)** y subir el **mismo**
   `taller-aws-devops-semana2-app.yaml`.
2. En **Stack name**, escribir `taller-aws-<su-nombre>-eco`.
3. Repetir los tres nombres de stack —red, datos, plataforma— sin cambiar ninguno: son
   las mismas dependencias.
4. Cambiar solo cuatro valores:
   - **ImageUri**: la **misma** imagen de siempre.
   - **ComandoContenedor**: `courses_server,echo`.
   - **RutaPath**: `/eco/*`.
   - **Prioridad**: `10`, más bajo que el `100` de la primera. La ruta específica se
     evalúa antes que el `/*` que atiende todo.
5. Aceptar la capacidad de IAM y esperar a **CREATE_COMPLETE**.

Después, un pedido cualquiera bajo `/eco/` devuelve un `echo` del request. Lo podemos
probar con `curl` o utilizando el widget a continuación.

```bash
curl -s "<UrlBase>/eco/prueba?x=1" | head -20
```

:::app
<cb-http endpoint="/eco/prueba?x=1"></cb-http>
:::

> Puede abrir la consola de desarrollo para ver más detalles.

::: extra Eco en desarrollo
Si la guía no está corriendo detrás del ALB del taller (por ejemplo, en el ambiente
local), no hay regla que desvíe `/eco/*` y contesta la guía misma con su 404. Ese
resultado también dice algo: muestra **quién** atendió el pedido. En ese caso se puede
escribir el dominio del ALB (el de la `<UrlBase>`) en el primer campo, que vacío
significa "mismo origen": el servidor de eco contesta con CORS abierto, así que el
navegador permite el pedido aunque la guía viva en otro origen.
:::

::: extra Tres direcciones de origen distintas, ¿cual sirve?

El bloque `network` contesta tres veces la pregunta "¿quién llamó?", y las tres
respuestas son distintas a propósito:

- **`peer`** es el otro extremo del socket TCP, lo único que el proceso ve por sí mismo.
  Detrás de un balanceador **es el balanceador**: `10.0.0.5` es una de sus interfaces,
  no el visitante.
- **`forwarded_for`** es la cadena de saltos que el pedido cruzó. El ALB agrega la IP de
  quien lo llamó al final del header `X-Forwarded-For`, así que el **primer** elemento es
  el cliente original.
- **`client_ip`** es la respuesta corta: el primer salto si hubo proxy, y el `peer` si no
  lo hubo. Es el valor que corresponde registrar en un log de acceso.

La advertencia va con el paquete: `X-Forwarded-For` es un header, y cualquiera puede
mandarlo. Solo vale lo que valga la cadena de proxies que lo sobrescribió. Delante del
ALB es confiable; expuesto directo a internet, no.

`local` cierra el cuadro desde el otro lado: en modo `awsvpc` es la **IP privada de la
tarea**, la misma que el target group tiene registrada. Ahí se ve, sin abstracciones,
que la tarea es un miembro más de la VPC.
:::

::: extra La red, desde adentro de la tarea
El bloque `ecs` no sale de ningún header: sale del **endpoint de metadata de la tarea**,
que ECS expone en cada tarea a través de la variable `ECS_CONTAINER_METADATA_URI_V4`. El
servidor lo consulta una vez al arrancar —esos datos no cambian mientras la tarea vive—
y si no está, contesta igual con `"ecs": null`.

Vale la pena porque convierte el dibujo de la Semana 1 en datos verificables:

- `availability_zone` y `subnet_cidr` dicen **en cuál de las dos subredes** cayó esta
  tarea. Con `DesiredCount` en 2 y varios pedidos seguidos, se ven las dos zonas
  alternándose: eso es la alta disponibilidad, medida en vez de prometida.
- `private_ipv4`, `subnet_gateway`, y `dns_servers` muestran que la tarea recibió una
  IP de la subred, la gateway de esa subred, y el resolver de la VPC —la base del rango
  de la VPC más dos, `10.0.0.2` para el `10.0.0.0/16` del taller—.
- `task_id`, `family`, y `revision` identifican **qué** contestó. Al desplegar una
  versión nueva, `revision` cambia sin que cambie nada más, y ese es el pulso del
  deploy.
:::

### La misma imagen, otro comando
:::inline-slide light with-title

Lo que convierte una imagen en dos aplicaciones distintas es una sola propiedad de la
task definition:

```yaml
Command: !If
  - ConComando
  - !Ref ComandoContenedor
  - !Ref AWS::NoValue
```

`ComandoContenedor` es de tipo `CommaDelimitedList`, así que `courses_server,echo` llega
como una lista de dos elementos y reemplaza el `CMD` del Dockerfile. Si el parámetro
queda vacío, `AWS::NoValue` **borra la propiedad entera** y la imagen arranca con su
comando de siempre. Sin ese `AWS::NoValue` se enviaría una lista vacía, que no es lo
mismo que no enviar nada: ECS la rechaza.
:::

Esto vale más allá del taller. Un mismo artefacto que corre como servidor web, como
worker de una cola, o como tarea programada, según el comando, es un patrón habitual —
y del lado de CloudFormation cuesta cinco líneas.

:::inline-slide
### Enrutar por nombre en vez de por ruta

:::skip
`/eco/*` funciona, pero obliga a que la aplicación viva bajo un prefijo. La alternativa
es darle **nombre propio**: `echo.<dominio>`, con el mismo balanceador. El template lo
soporta con el parámetro `NombreHost`, que cambia el tipo de condición de la regla:
:::

```yaml
Conditions:
  - !If
    - ConHost
    - Field: host-header
      HostHeaderConfig:
        Values: [!Ref NombreHost]
    - Field: path-pattern
      PathPatternConfig:
        Values: [!Ref RutaPath]
```
:::

Para que un nombre nuevo funcione hacen falta dos cosas del lado de la plataforma, y las
dos ya están resueltas allí: el certificado se pide **con comodín** (`*.<dominio>` como
`SubjectAlternativeNames`), y Route 53 lleva un registro alias comodín hacia el
balanceador. Con eso, cada aplicación nueva elige su subdominio sin tocar el stack de
plataforma. Con un certificado por nombre, en cambio, agregar una aplicación obligaría a
modificar —y volver a validar— el stack compartido.

Con `NombreHost=echo.<dominio>` y `UsarHttps=si`, el servidor de eco contesta en
`https://echo.<dominio>`. Si además el comando se lanza como
`courses_server,echo,--name,echo.<dominio>`, la respuesta trae
`"matched_public_name": true` cuando el pedido llegó por ese nombre, y `false` cuando
llegó por el DNS crudo del balanceador. Es una forma barata de comprobar que la regla
de host enruta lo que se cree que enruta.

:::inline-slide light
## Que mirar despues del despliegue

:::skip
Lo que hay que mirar después vale más que el despliegue en sí:
:::

- En **ECS → Clusters** hay **un** clúster, con **dos** servicios adentro.

:::skip
  ```bash
  export TALLER=taller-aws-<su-nombre>
  aws ecs list-services \
    --cluster "$TALLER-plataforma" \
    --query "serviceArns" --output text
  ```
:::

- En **EC2 → Load Balancers** hay **un** balanceador. Su listener del puerto 80 tiene
  ahora dos reglas, ordenadas por prioridad, más la acción por defecto.

:::skip
  ```bash
  LISTENER=$(aws cloudformation describe-stacks \
    --stack-name "$TALLER-plataforma" \
    --query "Stacks[0].Outputs[?OutputKey=='ListenerHttpArn'].OutputValue" \
    --output text)
  aws elbv2 describe-rules --listener-arn "$LISTENER" \
    --query "Rules[].{Prioridad:Priority,Regla:Conditions[0].Values[0]}" \
    --output table
  ```
:::

- En **CloudWatch → Log groups** hay dos grupos, `/ecs/taller-aws-<su-nombre>-app` y
  `/ecs/taller-aws-<su-nombre>-eco`. Salen separados solos, porque el template los
  nombra con `!Sub "/ecs/${AWS::StackName}"`.

:::skip
  ```bash
  aws logs describe-log-groups \
    --log-group-name-prefix "/ecs/$TALLER" \
    --query "logGroups[].logGroupName" --output text
  ```
:::

- Borrar el stack `-eco` deja el resto intacto. Ninguna otra pieza se enteró de que
  existió.

:::skip
  ```bash
  aws cloudformation delete-stack --stack-name "$TALLER-eco"
  aws cloudformation wait stack-delete-complete --stack-name "$TALLER-eco"
  aws cloudformation list-stacks \
    --stack-status-filter CREATE_COMPLETE UPDATE_COMPLETE IMPORT_COMPLETE \
    --query "StackSummaries[?starts_with(StackName, '$TALLER')].StackName" \
    --output text
  ```
:::

Y sobre todo: **ni una línea cambió** en los stacks de red, datos, o plataforma. Esa es
la prueba de que el corte quedó donde debía. Un corte mal puesto se delata al revés,
para agregar la segunda aplicación habría que editar el stack de la primera.
:::

:::inline-slide light
## Distribuir el patrón entre proyectos

:::skip
Lo anterior funcionó porque el template de aplicación quedó **portátil**, y eso no fue
casualidad. Dos propiedades lo hacen distribuible, y las dos se pueden verificar leyendo
el archivo:

- **Nada del ambiente está escrito adentro.** Cada dependencia llega como parámetro —
  cuatro nombres de stack—, no como un ARN fijo. El mismo archivo sirve en otra cuenta,
  otra región, u otro ambiente, sin editarlo.
- **Ningún nombre físico está fijo.** El grupo de logs, y todo lo demás que se nombra,
  derivan de `${AWS::StackName}`. Por eso dos copias del template conviven sin
  colisionar.
:::

:::add visibility=slide
Dos propiedades hacen el stack de aplicación distribuible:

1. **Nada del ambiente está escrito adentro.**
2. **Ningún nombre físico está fijo.**
:::

:::skip
Con eso, distribuirlo es publicarlo. El template va a un bucket de S3, versionado, y
cada proyecto lo lanza desde ahí sin copiarlo a su repositorio:
:::

:::add visibility=slide
Podemos publicarlo en S3 y consumirlo desde otros proyectos.
:::

```bash
# Quien mantiene el patrón lo publica, con versión en la ruta
aws s3 cp taller-aws-devops-semana2-app.yaml \
  s3://plantillas-plataforma/app/v3.yaml

# Cada proyecto lo consume, sin copiarlo
aws cloudformation create-stack \
  --stack-name taller-aws-maria-eco \
  --template-url https://plantillas-plataforma.s3.amazonaws.com/app/v3.yaml \
  --parameters ParameterKey=ImageUri,ParameterValue=... \
               ParameterKey=RutaPath,ParameterValue='/eco/*' \
  --capabilities CAPABILITY_IAM
```

::: info
La versión en la ruta es lo que hace usable el esquema: `v3.yaml` no cambia bajo los
pies de quien ya lo desplegó, y migrar a `v4` es una decisión de cada proyecto.
::: #info
::: #inline-slide

El límite es el mismo de siempre: los exports viven en **una cuenta y una región**. Una
plataforma compartida sirve a las aplicaciones de esa cuenta, y no más. Repartir un
mismo patrón entre varias cuentas es distribuir el **template**, no compartir los
recursos.

:::inline-slide light
## Formas de servir CloudFormation más alla de S3

:::skip
Publicar el template alcanza para un puñado de proyectos. Cuando son muchos, o cuando
hace falta gobernarlos, AWS ofrece tres mecanismos, de menor a mayor ceremonia:

- **Módulos de CloudFormation.** Un fragmento de template se registra en el registro
  privado de la cuenta como un tipo propio (ejemplo: `MiOrg::Taller::App::MODULE`) y desde
  entonces se usa como si fuera un recurso más. A diferencia del archivo en S3, el
  consumidor no ve las piezas de adentro: ve un recurso con sus propiedades. Los
  recursos del módulo terminan en el stack de quien lo usa, así que no hay un stack
  extra que coordinar.
- **Service Catalog.** El patrón se publica como un **producto** dentro de un
  portafolio, y el portafolio se comparte con cuentas u unidades organizativas. Quien
  lo lanza no necesita permisos sobre ECS ni sobre IAM: los toma prestados del rol del
  producto. Es la respuesta de AWS a "que treinta equipos puedan desplegar esto, pero
  solo esto, y solo así".
- **StackSets.** Un mismo template desplegado desde una cuenta central hacia muchas
  cuentas y regiones a la vez, con una sola operación. Sirve justamente para lo que los
  exports no pueden: la red compartida, la plataforma, o las políticas de base en cada
  cuenta de la organización.
:::

:::add visibility=slide
AWS Ofrece otros tres mecanismos:

- **Módulos de CloudFormation.**
- **Service Catalog.**
- **StackSets.**
:::

::: info
Los tres resuelven el mismo problema con distinta rigidez, y los tres siguen
desplegando stacks de CloudFormation. Lo aprendido en esta semana —change sets,
eventos, exports— no cambia.
:::
::: #inline-slide

## El patrón como módulo
:::inline-slide light with-title

Esta práctica registra el patrón de aplicación como el tipo
`CloudBridge::Taller::App::MODULE`, y vuelve a desplegar el eco como **un solo
recurso** de ese tipo.

```yaml
  Eco:
    Type: CloudBridge::Taller::App::MODULE
```
::: #inline-slide

:::inline-slide with-title light
El fragmento del módulo no es el template de aplicación copiado: dos reglas del
registro obligan a cambiarlo, y las dos enseñan algo.

- **Un fragmento no puede usar `Fn::ImportValue` ni `Export`.** Un módulo debe ser
  predecible: nada externo puede colarse adentro. Los nueve imports del template se
  vuelven **parámetros** del módulo, y el que importa es el consumidor:

  ```yaml
  # El template importaba…
  Cluster:
    Fn::ImportValue: !Sub "${PlataformaStackName}-cluster-nombre"

  # …el fragmento declara un parámetro, y el consumidor le pasa el valor
  Cluster: !Ref ClusterNombre
  ```

  Las referencias que cruzan deben quedar explicitas en el esquema del módulo.
::: #inline-slide

:::inline-slide with-title
- **Los parámetros de un módulo no validan restricciones.** `AllowedPattern`,
  `AllowedValues`, y `MinValue` no se aplican, así que las restricciones se
  recuperan en el template consumidor, que sí es un template normal.
:::

Un fragmento **no puede** usar `Fn::ImportValue` ni `Export`.

Cada import se vuelve un **parámetro**: el contrato queda explícito,
propiedad por propiedad. El que importa es el **consumidor**.

:::inline-slide light with-title
::: info
A diferencia de un stack anidado, un módulo no agrega un stack: al crear el change
set, CloudFormation lo expande y sus recursos aparecen **en el stack del
consumidor**, con el nombre del módulo como prefijo del ID lógico
(`Eco` → `EcoServicioApp`). Los nombres físicos siguen saliendo de
`${AWS::StackName}` (ahora el del consumidor), así que un stack llamado
`taller-aws-<su-nombre>-eco` produce exactamente los mismos nombres que el stack
clásico que se borró.
::: # info

Los dos archivos de la práctica:

:::app
<cb-file path="./infra/templates/taller-aws-devops-semana2-app-modulo-fragmento.yaml" type="yaml" toggleable full-path></cb-file>
:::

:::app
<cb-file path="./infra/templates/taller-aws-devops-semana2-eco-modulo.yaml" type="yaml" toggleable full-path></cb-file>
:::
::: #inline-slide

### La configuración también es contrato
:::inline-slide light with-title

:::skip
El template clásico traía el bloque `Environment` del contenedor escrito adentro, y
quien necesitaba otra configuración lo editaba. Un módulo no se edita: lo que se
puede configurar tiene que estar **en el esquema**, como una propiedad más. El
fragmento expone tres decisiones con nombre:


| Propiedad | Variable | Default |
| --- | --- | --- |
| `AppsGated` | `CB_APPS_GATED` | `all` |
| `AppsPublicCollections` | `CB_APPS_PUBLIC_COLLECTIONS` | `counters` |
| `AppsSecret` | `CB_APPS_SECRET` | vacío — la variable **no se define** |

Y lo estructural queda fijo a propósito: `PORT` va atado al puerto del target
group, y `CB_APPS_TABLE` a la tabla que entra por propiedad. Exponerlos sería
invitar a romper el módulo desde afuera. Esa es la diferencia con el archivo en
S3: el template expone todo lo que contiene; el módulo expone **lo que su autor
decidió**, y nada más.
:::

:::add visibility=slide
El bloque `Environment` no se edita: se expone **en el esquema**.

- `AppsGated` → `CB_APPS_GATED`
- `AppsPublicCollections` → `CB_APPS_PUBLIC_COLLECTIONS`
- `AppsSecret` → `CB_APPS_SECRET` (vacío: la variable no existe)

Lo estructural (`PORT`, `CB_APPS_TABLE`) queda fijo a propósito.
::: # add

La variable opcional reusa un truco ya visto. `Command` usaba `AWS::NoValue` para
borrar una propiedad entera; aquí borra **un elemento de una lista**:

```yaml
Environment:
  - Name: CB_APPS_GATED
    Value: !Ref AppsGated
  - !If
    - ConSecreto
    - Name: CB_APPS_SECRET
      Value: !Ref AppsSecret
    - !Ref AWS::NoValue
```

::: warning
En este taller el secreto es un **string simple**, a propósito: viaja en texto
plano y queda legible en la task definition. El `NoEcho` del template consumidor
solo lo oculta en la consola de CloudFormation. La forma seria `secrets` +
`valueFrom`, con el valor en Secrets Manager.
::: # warning
::: # inline-slide

:::inline-slide with-title
### ¿Y si la configuración viajara por el comando?

Hay una salida al límite de los arrays que no pasa por `Environment`: el
**comando**. El fragmento no cambia (`ComandoContenedor` sigue siendo un string
que se parte por comas). Es el consumidor el que escribe el comando como lista,
interpola lo sensible como **referencia**, y deja que `Fn::Join` lo aplane:

```yaml
Parameters:
  SecretARN:
    Type: String
    Description: Valor del secreto, ARN de Secrets Manager, o ARN de Parameter Store

Resources:
  Eco:
    Type: CloudBridge::Taller::App::MODULE
    Properties:
      ImageUri: !Ref ImageUri
      ComandoContenedor: !Join
        - ","
        - - courses_server
          - eco
          - !Sub "--secret=${SecretARN}"
      # ... el resto de las propiedades, igual que antes
```

:::skip
YAML no concatena listas; `Fn::Join` sí: la lista se escribe donde se lee, y el
string aparece recién al crear el change set. El `!Sub` va solo en el elemento
que interpola, anidado dentro del `!Join`. Si el argumento es opcional, el mismo
`Fn::If` + `AWS::NoValue` de la variable de entorno borra el elemento antes del
`Join`.

El parámetro admite el valor directo o un ARN: la aplicación decide mirando el
prefijo `arn:`. Un ARN no es secreto: puede quedar legible en la task definition
sin exponer nada. La aplicación lo resuelve al arrancar, con el SDK, contra
Secrets Manager o Parameter Store. Las dos estabilidades que se ganan: el módulo no necesita
versiones nuevas —configuración nueva es un argumento nuevo—, y cambiar un valor
detrás de la referencia ni siquiera toca CloudFormation: `aws ssm put-parameter`
y un `--force-new-deployment` del servicio.

El costo no desaparece: **se muda a IAM**. El task role del fragmento tendría
que poder leer esos ARNs, así que el esquema necesitaría una propiedad más —una
lista de ARNs permitidos que el fragmento convierte en policy—. El contrato no
se achica; cambia de forma. Y la aplicación carga con código que ECS ofrece
gratis: `secrets` + `valueFrom` hace exactamente esa resolución, sin SDK ni
argumentos, y se ve en la siguiente sección. La variante de Parameter Store como
acoplamiento entre stacks aparece más abajo, en el refactoring a escala.
:::
::: #inline-slide

:::slide
## La práctica: el patrón como módulo

1. **Registrar** el tipo con la CLI de CloudFormation, en CloudShell.
2. **Leer** el contrato en el registro.
3. **Recrear** el eco: un stack, un recurso.
4. **Mirar** dónde quedaron los recursos.
5. **Publicar** una segunda versión.

:::app
<cb-goto path="Práctica guiada: el patrón como módulo"></cb-goto>
::: # app
:::

## Práctica guiada: el patrón como módulo

### Registrar el tipo

1. Abrir [**CloudShell**](https://console.aws.amazon.com/cloudshell/home). Todo el
   registro se hace ahí: trae Python, `pip`, y las credenciales del pod ya
   resueltas.
2. Instalar la [**CLI de CloudFormation**](https://docs.aws.amazon.com/cloudformation-cli/latest/userguide/what-is-cloudformation-cli.html)
   (CFN-CLI 2.0) —una herramienta aparte de la `awscli`, hecha para desarrollar
   extensiones del registro—. Para módulos no hace falta ningún plugin de
   lenguaje; `setuptools` sí, porque el Python de CloudShell ya no lo trae:

   ```bash
   pip3 install cloudformation-cli "setuptools<81"
   ```

3. Crear el proyecto del módulo. `cfn init` pregunta qué se desarrolla
   (contestar `m`, módulo) y el nombre del tipo (contestar
   `CloudBridge::Taller::App::MODULE`):

   ```bash
   mkdir app-modulo && cd app-modulo
   cfn init
   ```

4. Reemplazar el fragmento de ejemplo por el del taller:

   ```bash
   rm fragments/sample.json
   curl -o fragments/app-modulo.yaml \
     https://raw.githubusercontent.com/cloudbridgeuy/courses/main/infra/templates/taller-aws-devops-semana2-app-modulo-fragmento.yaml
   ```

5. Registrar. `cfn submit` valida el fragmento, genera el esquema, y sube el tipo
   al registro de la cuenta:

   ```bash
   cfn submit
   ```

6. Verificar que el tipo existe:

   ```bash
   aws cloudformation list-types --visibility PRIVATE \
     --query "TypeSummaries[].{Tipo:TypeName,Version:DefaultVersionId}" \
     --output table
   ```

::: info
`cfn submit` necesita un bucket donde subir el paquete, y lo resuelve creando un
stack propio: `CloudFormationManagedUploadInfrastructure`. Aparece en la lista de
stacks y es normal: es infraestructura de la herramienta, no del taller.
:::

### Leer el contrato

1. Abrir [**CloudFormation → Registry → Activated extensions**](https://console.aws.amazon.com/cloudformation/home#/registry/activated),
   pestaña **Modules**, y entrar a `CloudBridge::Taller::App::MODULE`.
2. La pestaña **Schema** muestra lo que el consumidor ve: las propiedades —una por
   parámetro del fragmento— y los recursos a los que el módulo se resuelve. El
   mismo esquema, desde la terminal:

   ```bash
   aws cloudformation describe-type --type MODULE \
     --type-name CloudBridge::Taller::App::MODULE \
     --query "Schema" --output text | head -30
   ```

### Recrear el eco

1. Pulsar **Create stack → With new resources (standard)** y subir
   `taller-aws-devops-semana2-eco-modulo.yaml`.
2. En **Stack name**, escribir `taller-aws-<su-nombre>-modulo-eco`.
3. Completar **ImageUri** con la imagen de siempre, y los tres nombres de stack
   (red, datos, plataforma). El resto ya viene con los valores del eco:
   `ComandoContenedor=courses_server,echo`, `RutaPath=/eco2/*`, `Prioridad=10`.
4. Aceptar la capacidad de IAM y esperar a **CREATE_COMPLETE**.
5. Probar que el eco contesta, igual que la primera vez:

   ```bash
   curl -s "<UrlBase>/eco2/prueba?x=1" | head -20
   ```

:::app
<cb-http endpoint="/eco2/prueba?x=1"></cb-http>
:::

### Mirar dónde quedaron los recursos

En la pestaña **Resources** del stack están el servicio, la task definition, la
regla, y el resto —los mismos nueve recursos del stack clásico—, todos con el
prefijo `Eco` en el ID lógico. No hay ningún stack anidado. La misma vista, desde
la terminal:

```bash
export TALLER=taller-aws-<su-nombre>
aws cloudformation describe-stack-resources \
  --stack-name "$TALLER-eco" \
  --query "StackResources[].{Logico:LogicalResourceId,Tipo:ResourceType,Modulo:ModuleInfo.TypeHierarchy}" \
  --output table
```

La columna `Modulo` dice de qué tipo salió cada recurso: esa es la traza que queda
después de la expansión. Y el grupo de logs se llama `/ecs/taller-aws-<su-nombre>-eco`,
como siempre, porque `${AWS::StackName}` resolvió al stack consumidor:

```bash
aws logs describe-log-groups \
  --log-group-name-prefix "/ecs/$TALLER-eco" \
  --query "logGroups[].logGroupName" --output text
```

Y la configuración quedó como el esquema la prometía. La task definition se
resuelve por su ID lógico —con el prefijo del módulo—, y adentro están las
variables:

```bash
TAREA=$(aws cloudformation describe-stack-resources \
  --stack-name "$TALLER-eco" \
  --logical-resource-id EcoTareaApp \
  --query "StackResources[0].PhysicalResourceId" --output text)
aws ecs describe-task-definition --task-definition "$TAREA" \
  --query "taskDefinition.containerDefinitions[0].environment" \
  --output table
```

`CB_APPS_GATED` y `CB_APPS_PUBLIC_COLLECTIONS` traen los defaults del módulo, y
`CB_APPS_SECRET` **no aparece**: `AppsSecret` quedó vacío, así que `AWS::NoValue`
borró el elemento antes de que la lista llegara a ECS.

### Publicar una segunda versión

Un módulo no se edita: se **versiona**. Cada `cfn submit` publica una versión
nueva, y los stacks existentes no se enteran.

1. En CloudShell, subir la retención de logs del fragmento, de `7` a `14` días:

   ```bash
   sed -i 's/RetentionInDays: 7/RetentionInDays: 14/' fragments/app-modulo.yaml
   cfn submit
   ```

2. Listar las versiones. La nueva existe, y la default sigue siendo la primera:

   ```bash
   aws cloudformation list-type-versions --type MODULE \
     --type-name CloudBridge::Taller::App::MODULE \
     --query "TypeVersionSummaries[].{Version:VersionId,Default:IsDefaultVersion}" \
     --output table
   ```

3. Promover la nueva versión a default:

   ```bash
   aws cloudformation set-type-default-version --type MODULE \
     --type-name CloudBridge::Taller::App::MODULE \
     --version-id "00000002"
   ```

4. El stack del eco sigue en la versión con la que se creó. Migra recién en su
   próximo update, cuando su change set vuelva a expandir el módulo —y ese change
   set muestra el cambio de retención antes de aplicarlo.

::: info
Es la misma disciplina que el `v3.yaml`/`v4.yaml` del bucket de S3: la versión
publicada no cambia bajo los pies de nadie, y migrar es una decisión de cada
consumidor. La diferencia es quién la hace cumplir —allá una convención de rutas,
acá el registro—.
:::

:::slide light
## Lo que el módulo cambió

| | Template en S3 | Módulo |
| --- | --- | --- |
| El consumidor ve | El archivo entero | Un recurso con propiedades |
| El contrato | Parámetros + convención | Esquema en el registro |
| Los recursos viven | En su propio stack | En el stack del consumidor |
| Versionar | Convención de rutas | El registro lo hace cumplir |

Mismo patrón, distinta rigidez.
:::

## A escala: stack refactoring

La secuencia manual —`Retain`, huérfano, import— muestra la mecánica real, y para un
recurso es perfectamente manejable. Para mover decenas de recursos entre stacks, AWS
ofrece una operación que hace los tres pasos de forma atómica: el **stack
refactoring** (`CreateStackRefactor`). Se le entregan los templates finales de ambos
stacks y un mapa de qué recurso va a dónde; CloudFormation valida que ningún recurso
físico se toque, y ejecuta la mudanza completa —incluyendo renombres de IDs lógicos—
en una sola operación revisable, al estilo de un change set.

::: extra Adoptar infraestructura que nació a mano

El import tiene un segundo uso, más frecuente que las migraciones entre stacks: al
mecanismo le da igual *quién* creó el recurso. Una tabla creada a mano en la consola
hace tres años se importa exactamente igual que la tabla huérfana de esta práctica.
Esto convierte al import en la puerta de entrada a IaC para infraestructura que nació
sin templates.

Para no escribir esos templates a mano, la consola de CloudFormation incluye el
**IaC generator**: escanea la cuenta, descubre los recursos que ningún stack
gestiona, y genera el template por ellos, dejándolo listo para el flujo de import.
Advertencias de uso: el template generado sale con valores fijos que conviene
parametrizar, las propiedades de solo escritura (como un secreto) no se pueden leer
de vuelta, no todos los tipos de recurso están soportados, y después de importar
conviene correr **Detect drift** para confirmar que template y realidad coinciden.
:::

::: extra La otra forma de componer: stacks anidados
Exports e imports no son la única manera de partir un template grande. La alternativa
son los **stacks anidados**: un template padre que declara a sus hijos como recursos
de tipo `AWS::CloudFormation::Stack`.

```yaml
  Red:
    Type: AWS::CloudFormation::Stack
    Properties:
      TemplateURL: https://s3.amazonaws.com/mi-bucket/red.yaml

  App:
    Type: AWS::CloudFormation::Stack
    Properties:
      TemplateURL: https://s3.amazonaws.com/mi-bucket/app.yaml
      Parameters:
        VpcId: !GetAtt Red.Outputs.VpcId     # el output del hijo, sin export
```

El hijo pasa sus valores por `!GetAtt <Hijo>.Outputs.<Nombre>`, sin exports de por
medio. Los dos modelos resuelven el mismo problema con filosofías opuestas:

| | Stacks separados (exports) | Stacks anidados |
| --- | --- | --- |
| Se despliegan | Por separado, en su orden | Todos juntos, desde el padre |
| Ciclo de vida | Independiente por stack | Uno solo, el del padre |
| Acoplamiento | Bajo: el contrato es el export | Alto: el padre conoce a todos |
| Templates | En disco, se suben a mano | En S3, obligatorio |

La elección sigue al ciclo de vida, que es el criterio de toda esta sección. Si las
partes cambian a ritmos distintos y las gestionan equipos distintos —red, datos,
aplicación— van en stacks separados. Si son piezas de una sola unidad que siempre se
despliega junta, anidarlas evita coordinar tres operaciones para un solo cambio. El
taller usa stacks separados porque su tema **es** la diferencia de ciclos de vida.
:::

::: extra Acoplamiento flojo con Parameter Store
Hay una tercera vía, entre el export rígido y el anidamiento. El stack productor
escribe su valor en **SSM Parameter Store** como un recurso más, y el consumidor lo lee
como parámetro:

```yaml
  # En el stack de red: publicar el ID de la VPC
  ParametroVpc:
    Type: AWS::SSM::Parameter
    Properties:
      Name: /taller/red/vpc-id
      Type: String
      Value: !Ref VpcApp
```

```yaml
  # En el stack de aplicación: leerlo
Parameters:
  VpcId:
    Type: AWS::SSM::Parameter::Value<String>
    Default: /taller/red/vpc-id
```

El tipo `AWS::SSM::Parameter::Value<String>` hace que CloudFormation resuelva el
parámetro contra Parameter Store al lanzar el stack, y `VpcId` llegue con el valor, no
con la ruta. Frente a los exports gana en flexibilidad: el valor se puede cambiar sin
borrar a nadie, y funciona entre regiones y entre cuentas. Y pierde justo en lo mismo:
nadie impide borrar el stack de red mientras la aplicación depende de él, porque no hay
contrato que hacer cumplir. Se elige según lo que duela más, un cambio bloqueado o un
borrado no detectado.
:::

---

{#ejercicio-11}
### Ejercicio 11 — Migrar la tabla a su propio stack

Partiendo del stack monolítico de la Semana 1, dejar el ambiente corriendo en cuatro
stacks separados —red, plataforma, datos, aplicación— sin perder los datos de la tabla.
Escribir un contador antes de empezar y demostrar, al final, que sigue ahí.

::: solucion
1. Resolver el nombre físico de la tabla con
   `aws cloudformation describe-stack-resources` y guardarlo en `$TABLA`.
2. Escribir un contador de prueba con la app `counter` de la guía servida por el
   propio stack (el widget ejecuta un `UpdateItem` con `ADD` sobre
   `collection = counters`, `key = migracion`).
3. Agregar `DeletionPolicy: Retain` a `TablaApp` en
   `taller-aws-devops-semana1.yaml` y aplicarlo con un change set.
4. Borrar el stack `taller-aws-<su-nombre>`. En **Events**, la tabla queda como
   **DELETE_SKIPPED**; verificar con `aws dynamodb describe-table` que sigue
   `ACTIVE`.
5. Crear `taller-aws-<su-nombre>-red` con `taller-aws-devops-semana2-red.yaml`
   (sin parámetros).
6. Crear `taller-aws-<su-nombre>-plataforma` con
   `taller-aws-devops-semana2-plataforma.yaml`, indicando el stack de red y dejando
   los dos parámetros de dominio vacíos. La **UrlBase** de sus outputs debe responder
   `404`: el balanceador existe y ninguna regla lo enruta todavía.
7. Crear `taller-aws-<su-nombre>-datos` con **Create stack → With existing resources
   (import resources)**, subiendo `taller-aws-devops-semana2-datos-import.yaml` (el
   import no acepta `Outputs`) y pegando `$TABLA` como **TableName**. Esperar a
   **IMPORT_COMPLETE**. Después, aplicar `taller-aws-devops-semana2-datos.yaml`
   con `aws cloudformation update-stack` (update directo: un change set con
   cambios solo en `Outputs` reporta "didn't contain changes") para agregar los
   dos exports.
8. Crear `taller-aws-<su-nombre>-app` con `taller-aws-devops-semana2-app.yaml`,
   completando el URI de la imagen y los nombres de los stacks de red, datos, y
   plataforma. Aceptar la capacidad de IAM.
9. Volver a abrir la **UrlBase** del stack de plataforma: ahora carga la guía. Releer
   el contador con `aws dynamodb get-item`: el valor escrito en el paso 2 sigue ahí.
:::

:::slide light
{{ejercicio-11}}
:::
