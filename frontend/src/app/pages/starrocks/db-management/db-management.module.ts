import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { NbCardModule, NbButtonModule, NbInputModule, NbSelectModule, NbCheckboxModule, NbSpinnerModule, NbIconModule, NbTabsetModule } from '@nebular/theme';
import { Ng2SmartTableModule } from 'ng2-smart-table';
import { DbManagementRoutingModule } from './db-management-routing.module';
import { DbManagementComponent } from './db-management.component';
import { DatabasesComponent } from './databases/databases.component';
import { TablesComponent } from './tables/tables.component';
import { ConfirmDialogComponent } from '../../../@core/components/confirm-dialog/confirm-dialog.component';

@NgModule({
  declarations: [
    DbManagementComponent,
    DatabasesComponent,
    TablesComponent
  ],
  imports: [
    CommonModule,
    FormsModule,
    DbManagementRoutingModule,
    NbCardModule,
    NbButtonModule,
    NbInputModule,
    NbSelectModule,
    NbCheckboxModule,
    NbSpinnerModule,
    NbIconModule,
    NbTabsetModule,
    Ng2SmartTableModule
  ],
  entryComponents: [
    ConfirmDialogComponent
  ]
})
export class DbManagementModule { }
